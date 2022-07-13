use std::ops::Deref;

use crate::abci::messages::{
    BlockBeginRequest, BlockBeginResponse, BlockEndRequest, BlockEndResponse, InitChainRequest,
    InitChainResponse,
};
use crate::block::{BlockExecutionContext, BlockInfo};
use rs_drive::grovedb::TransactionArg;
use crate::execution::epoch_change::epoch::EpochInfo;

use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::platform::Platform;

pub trait TenderdashAbci {
    fn init_chain(
        &self,
        request: InitChainRequest,
        transaction: TransactionArg,
    ) -> Result<InitChainResponse, Error>;

    fn block_begin(
        &self,
        request: BlockBeginRequest,
        transaction: TransactionArg,
    ) -> Result<BlockBeginResponse, Error>;

    fn block_end(
        &self,
        request: BlockEndRequest,
        transaction: TransactionArg,
    ) -> Result<BlockEndResponse, Error>;
}

impl TenderdashAbci for Platform {
    fn init_chain(
        &self,
        _request: InitChainRequest,
        transaction: TransactionArg,
    ) -> Result<InitChainResponse, Error> {
        self.drive
            .create_initial_state_structure(transaction)
            .map_err(Error::Drive)?;

        let response = InitChainResponse {};

        Ok(response)
    }

    fn block_begin(
        &self,
        request: BlockBeginRequest,
        transaction: TransactionArg,
    ) -> Result<BlockBeginResponse, Error> {
        // Set genesis time
        let genesis_time = if request.block_height == 1 {
            self.drive.init_genesis(request.block_time_ms, transaction)?;
            request.block_time_ms
        } else {
            self.drive.get_genesis_time(transaction).map_err(Error::Drive)?.ok_or(Error::Execution(ExecutionError::DriveIncoherence("the genesis time must be set")))?
        };

        // Init block execution context
        let epoch_info = EpochInfo::calculate(
            genesis_time,
            request.block_time_ms,
            request.previous_block_time_ms,
        )?;

        let block_execution_context = BlockExecutionContext {
            block_info: BlockInfo::from_block_begin_request(&request),
            epoch_info,
        };

        self.block_execution_context
            .replace(Some(block_execution_context));

        let response = BlockBeginResponse {};

        Ok(response)
    }

    fn block_end(
        &self,
        request: BlockEndRequest,
        transaction: TransactionArg,
    ) -> Result<BlockEndResponse, Error> {
        // Retrieve block execution context
        let block_execution_context = self.block_execution_context.borrow();
        let block_execution_context = match block_execution_context.deref() {
            Some(block_execution_context) => block_execution_context,
            None => {
                return Err(Error::Execution(
                    ExecutionError::CorruptedCodeExecution(
                        "block execution context must be set in block begin handler",
                    ),
                ))
            }
        };

        // Process fees
        let distribution_info = self.process_block_fees(
            &block_execution_context.block_info,
            &block_execution_context.epoch_info,
            &request.fees,
            transaction,
        )?;

        let response = BlockEndResponse {
            current_epoch_index: block_execution_context.epoch_info.current_epoch_index,
            is_epoch_change: block_execution_context.epoch_info.is_epoch_change,
            masternodes_paid_count: distribution_info.masternodes_paid_count,
            paid_epoch_index: distribution_info.paid_epoch_index,
        };

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    mod handlers {
        use crate::common::helpers::fee_pools::{
            create_test_masternode_share_identities_and_documents,
        };
        use chrono::{Duration, Utc};
        use rust_decimal::prelude::ToPrimitive;
        use rs_drive::common::helpers::identities::create_test_masternode_identities;
        use crate::abci::handlers::TenderdashAbci;

        use crate::abci::messages::{BlockBeginRequest, BlockEndRequest, FeesAggregate, InitChainRequest};
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use crate::error::Error;
        use crate::error::execution::ExecutionError;

        #[test]
        fn test_abci_flow() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            // init chain
            let init_chain_request = InitChainRequest {};

            platform.init_chain(init_chain_request, Some(&transaction))
                .expect("should init chain");

            // setup the contract
            let contract = platform.create_mn_shares_contract(Some(&transaction));

            let genesis_time = Utc::now();

            let total_days = 22;

            let epoch_1_start_day = 20;

            let proposers_count = total_days;

            let storage_fees_per_block = 42000;

            // and create masternode identities
            let proposers =
                create_test_masternode_identities(&platform.drive, proposers_count, Some(&transaction));

            create_test_masternode_share_identities_and_documents(
                &platform.drive,
                &contract,
                &proposers,
                Some(&transaction),
            );

            // process blocks
            for day in 1..=total_days {
                let block_time = if day == 1 {
                    genesis_time
                } else {
                    genesis_time + Duration::days(day as i64 - 1)
                };

                let previous_block_time_ms = if day == 1 {
                    None
                } else {
                    Some((genesis_time + Duration::days(day as i64 - 2)).timestamp_millis().to_u64().expect("block time can not be before 1970"))
                };

                let block_height = day as u64;

                let block_time_ms = block_time.timestamp_millis().to_u64().expect("block time can not be before 1970");
                // Processing block
                let block_begin_request = BlockBeginRequest {
                    block_height,
                    block_time_ms,
                    previous_block_time_ms,
                    proposer_pro_tx_hash: proposers[day as usize - 1],
                };

                platform.block_begin(block_begin_request, Some(&transaction))
                    .expect(format!("should begin process block #{}", day).as_str());

                let block_end_request = BlockEndRequest {
                    fees: FeesAggregate {
                        processing_fees: 1600,
                        storage_fees: storage_fees_per_block,
                        refunds_by_epoch: vec![(1, 100)], // we are refunding 100 credits from epoch 1
                    },
                };

                let block_end_response = platform.block_end(block_end_request, Some(&transaction))
                    .expect(format!("should end process block #{}", day).as_str());

                // Should calculate correct current epochs
                let epoch_index = if day >= epoch_1_start_day { 1 } else { 0 };

                assert_eq!(block_end_response.current_epoch_index, epoch_index);

                assert_eq!(
                    block_end_response.is_epoch_change,
                    previous_block_time_ms.is_none() || day == epoch_1_start_day
                );

                // Should pay to 19 masternodes, when epochs 1 started
                let masternodes_paid_count = if day == epoch_1_start_day {
                    day as u16 - 1
                } else {
                    0
                };

                assert_eq!(
                    block_end_response.masternodes_paid_count,
                    masternodes_paid_count
                );

                // Should pay for the epochs 0, when epochs 1 started
                match block_end_response.paid_epoch_index {
                    Some(index) => assert_eq!(
                        index, 0,
                        "should pay to masternodes only when epochs 1 started"
                    ),
                    None => assert_ne!(
                        day, epoch_1_start_day,
                        "should pay to masternodes only when epochs 1 started"
                    ),
                }
            }

            let storage_fee_pool_value = platform.drive
                .get_aggregate_storage_fees_in_current_distribution_pool(Some(&transaction))
                .expect("should get storage fee pool");

            assert_eq!(
                storage_fee_pool_value,
                storage_fees_per_block * (total_days - epoch_1_start_day + 1) as u64,
                "should contain only storage fees from the last block"
            );
        }
    }
}
