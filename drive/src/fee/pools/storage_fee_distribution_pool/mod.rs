use crate::drive::Drive;
use grovedb::{Element, TransactionArg};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::pools::epoch::epoch_pool::EpochPool;
use crate::fee::pools::fee_pools::FeePools;

use super::constants;

pub struct StorageFeeDistributionPool {}

impl StorageFeeDistributionPool {
    pub fn distribute(
        &self,
        drive: &Drive,
        epoch_index: u16,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let storage_distribution_fees = Decimal::new(self.value(drive, transaction)?, 0);

        // a separate buffer from which we withdraw to correctly calculate fee share
        let mut storage_distribution_fees_buffer = storage_distribution_fees.clone();

        if storage_distribution_fees == dec!(0.0) {
            return Ok(());
        }

        for year in 0..50u16 {
            let distribution_percent = constants::FEE_DISTRIBUTION_TABLE[year as usize];

            let year_fee_share = storage_distribution_fees * distribution_percent;
            let epoch_fee_share = year_fee_share / dec!(20.0);

            let starting_epoch_index = epoch_index + year * 20;

            for index in starting_epoch_index..starting_epoch_index + 20 {
                let epoch_pool = EpochPool::new(index, drive);

                let storage_fee = epoch_pool.get_storage_fee(transaction)?;

                epoch_pool.update_storage_fee(storage_fee + epoch_fee_share, transaction)?;

                storage_distribution_fees_buffer -= epoch_fee_share;
            }
        }

        self.update(
            drive,
            storage_distribution_fees_buffer.try_into().map_err(|_| {
                Error::Fee(FeeError::CorruptedStorageFeePoolInvalidItemLength(
                    "fee pools storage fee pool is not i64",
                ))
            })?,
            transaction,
        )
    }

    pub fn update(
        &self,
        drive: &Drive,
        storage_fee: i64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        drive
            .grove
            .insert(
                FeePools::get_path(),
                constants::KEY_STORAGE_FEE_POOL.as_bytes(),
                Element::Item(storage_fee.to_le_bytes().to_vec(), None),
                transaction,
            )
            .map_err(Error::GroveDB)
    }

    pub fn value(&self, drive: &Drive, transaction: TransactionArg) -> Result<i64, Error> {
        let element = drive
            .grove
            .get(
                FeePools::get_path(),
                constants::KEY_STORAGE_FEE_POOL.as_bytes(),
                transaction,
            )
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            let fee = i64::from_le_bytes(item.as_slice().try_into().map_err(|_| {
                Error::Fee(FeeError::CorruptedStorageFeePoolInvalidItemLength(
                    "fee pools storage fee pool is not i64",
                ))
            })?);

            Ok(fee)
        } else {
            Err(Error::Fee(FeeError::CorruptedStorageFeePoolNotItem(
                "fee pools storage fee pool must be an item",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fee::pools::tests::helpers::setup::{
        setup_drive, setup_fee_pools, SetupFeePoolsOptions,
    };
    use crate::{
        error::{self, fee::FeeError},
        fee::pools::{constants, fee_pools::FeePools},
    };
    use grovedb::Element;

    mod helpers {
        use crate::drive::Drive;
        use crate::fee::pools::epoch::epoch_pool::EpochPool;
        use grovedb::TransactionArg;
        use rust_decimal::Decimal;

        pub fn get_storage_fees_from_epoch_pools(
            drive: &Drive,
            epoch_index: u16,
            transaction: TransactionArg,
        ) -> Vec<Decimal> {
            (epoch_index..epoch_index + 1000)
                .map(|index| {
                    let epoch_pool = EpochPool::new(index, &drive);
                    epoch_pool
                        .get_storage_fee(transaction)
                        .expect("to get storage fee")
                })
                .collect()
        }
    }

    mod distribute {
        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;

        use crate::fee::pools::epoch::epoch_pool::EpochPool;
        use crate::fee::pools::storage_fee_distribution_pool::tests::helpers;
        use crate::fee::pools::tests::helpers::setup::{setup_drive, setup_fee_pools};

        #[test]
        fn test_nothing_to_distribute() {
            todo!();
        }

        #[test]
        fn test_distribution_overflow() {
            let drive = setup_drive();
            let (transaction, fee_pools) = setup_fee_pools(&drive, None);

            let storage_pool = i64::MAX;
            let epoch_index = 0;

            fee_pools
                .storage_fee_distribution_pool
                .update(&drive, storage_pool, Some(&transaction))
                .expect("to update storage fee pool");

            fee_pools
                .storage_fee_distribution_pool
                .distribute(&drive, epoch_index, Some(&transaction))
                .expect("to distribute storage fee pool");

            // check leftover
            let storage_fee_pool_leftover = fee_pools
                .storage_fee_distribution_pool
                .value(&drive, Some(&transaction))
                .expect("to get storage fee pool");

            assert_eq!(storage_fee_pool_leftover, 0);
        }

        #[test]
        fn test_distribution() {
            let drive = setup_drive();
            let (transaction, fee_pools) = setup_fee_pools(&drive, None);

            let storage_pool = 1000;
            let epoch_index = 42;

            // init additional epoch pools as it will be done in epoch_change
            for i in 1000..=1000 + epoch_index {
                let epoch = EpochPool::new(i, &drive);
                epoch
                    .init_empty(Some(&transaction))
                    .expect("to init additional epoch pool");
            }

            fee_pools
                .storage_fee_distribution_pool
                .update(&drive, storage_pool, Some(&transaction))
                .expect("to update storage fee pool");

            fee_pools
                .storage_fee_distribution_pool
                .distribute(&drive, epoch_index, Some(&transaction))
                .expect("to distribute storage fee pool");

            // check leftover
            let storage_fee_pool_leftover = fee_pools
                .storage_fee_distribution_pool
                .value(&drive, Some(&transaction))
                .expect("to get storage fee pool");

            assert_eq!(storage_fee_pool_leftover, 0);

            // collect all the storage fee values of the 1000 epoch pools
            let storage_fees =
                helpers::get_storage_fees_from_epoch_pools(&drive, epoch_index, Some(&transaction));

            // compare them with reference table
            #[rustfmt::skip]
            let reference_fees = [
                dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000),
                dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000), dec!(2.5000),
                dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000),
                dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000), dec!(2.4000),
                dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000),
                dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000), dec!(2.3000),
                dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000),
                dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000), dec!(2.2000),
                dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000),
                dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000), dec!(2.1000),
                dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000),
                dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000), dec!(2.0000),
                dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250),
                dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250), dec!(1.9250),
                dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500),
                dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500), dec!(1.8500),
                dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750),
                dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750), dec!(1.7750),
                dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000),
                dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000), dec!(1.7000),
                dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250),
                dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250), dec!(1.6250),
                dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500),
                dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500), dec!(1.5500),
                dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750),
                dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750), dec!(1.4750),
                dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250),
                dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250), dec!(1.4250),
                dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750),
                dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750), dec!(1.3750),
                dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250),
                dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250), dec!(1.3250),
                dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750),
                dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750), dec!(1.2750),
                dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250),
                dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250), dec!(1.2250),
                dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750),
                dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750), dec!(1.1750),
                dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250),
                dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250), dec!(1.1250),
                dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750),
                dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750), dec!(1.0750),
                dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250),
                dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250), dec!(1.0250),
                dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750),
                dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750), dec!(0.9750),
                dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375),
                dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375), dec!(0.9375),
                dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000),
                dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000), dec!(0.9000),
                dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625),
                dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625), dec!(0.8625),
                dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250),
                dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250), dec!(0.8250),
                dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875),
                dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875), dec!(0.7875),
                dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500),
                dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500), dec!(0.7500),
                dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125),
                dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125), dec!(0.7125),
                dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750),
                dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750), dec!(0.6750),
                dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375),
                dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375), dec!(0.6375),
                dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000),
                dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000), dec!(0.6000),
                dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625),
                dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625), dec!(0.5625),
                dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250),
                dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250), dec!(0.5250),
                dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875),
                dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875), dec!(0.4875),
                dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500),
                dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500), dec!(0.4500),
                dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125),
                dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125), dec!(0.4125),
                dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750),
                dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750), dec!(0.3750),
                dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375),
                dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375), dec!(0.3375),
                dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000),
                dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000), dec!(0.3000),
                dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625),
                dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625), dec!(0.2625),
                dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375),
                dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375), dec!(0.2375),
                dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125),
                dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125), dec!(0.2125),
                dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875),
                dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875), dec!(0.1875),
                dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625),
                dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625), dec!(0.1625),
                dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375),
                dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375), dec!(0.1375),
                dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125),
                dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125), dec!(0.1125),
                dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875),
                dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875), dec!(0.0875),
                dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625),
                dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625), dec!(0.0625),
            ];

            assert_eq!(storage_fees, reference_fees);

            // refill storage fee pool once more
            fee_pools
                .storage_fee_distribution_pool
                .update(&drive, storage_pool, Some(&transaction))
                .expect("to update storage fee pool");

            // distribute fees once more
            fee_pools
                .storage_fee_distribution_pool
                .distribute(&drive, epoch_index, Some(&transaction))
                .expect("to distribute storage fee pool");

            // collect all the storage fee values of the 1000 epoch pools again
            let storage_fees =
                helpers::get_storage_fees_from_epoch_pools(&drive, epoch_index, Some(&transaction));

            // assert that all the values doubled meaning that distribution is repoducable
            assert_eq!(
                storage_fees,
                reference_fees
                    .iter()
                    .map(|val| val * dec!(2))
                    .collect::<Vec<Decimal>>()
            );
        }
    }

    #[test]
    fn test_update_and_value() {
        let drive = setup_drive();
        let (transaction, fee_pools) = setup_fee_pools(
            &drive,
            Some(SetupFeePoolsOptions {
                init_fee_pools: false,
            }),
        );

        let storage_fee = 42;

        match fee_pools
            .storage_fee_distribution_pool
            .value(&drive, Some(&transaction))
        {
            Ok(_) => assert!(
                false,
                "should not be able to get genesis time on uninit fee pools"
            ),
            Err(e) => match e {
                error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                _ => assert!(false, "invalid error type"),
            },
        }

        match fee_pools.storage_fee_distribution_pool.update(
            &drive,
            storage_fee,
            Some(&transaction),
        ) {
            Ok(_) => assert!(
                false,
                "should not be able to update genesis time on uninit fee pools"
            ),
            Err(e) => match e {
                error::Error::GroveDB(grovedb::Error::InvalidPath(_)) => assert!(true),
                _ => assert!(false, "invalid error type"),
            },
        }

        fee_pools
            .init(&drive, Some(&transaction))
            .expect("to init fee pools");

        fee_pools
            .storage_fee_distribution_pool
            .update(&drive, storage_fee, Some(&transaction))
            .expect("to update storage fee pool");

        let stored_storage_fee = fee_pools
            .storage_fee_distribution_pool
            .value(&drive, Some(&transaction))
            .expect("to get storage fee pool");

        assert_eq!(storage_fee, stored_storage_fee);

        drive
            .grove
            .insert(
                FeePools::get_path(),
                constants::KEY_STORAGE_FEE_POOL.as_bytes(),
                Element::Item(u128::MAX.to_le_bytes().to_vec(), None),
                Some(&transaction),
            )
            .expect("to insert invalid data");

        match fee_pools
            .storage_fee_distribution_pool
            .value(&drive, Some(&transaction))
        {
            Ok(_) => assert!(false, "should not be able to decode stored value"),
            Err(e) => match e {
                error::Error::Fee(FeeError::CorruptedStorageFeePoolInvalidItemLength(_)) => {
                    assert!(true)
                }
                _ => assert!(false, "ivalid error type"),
            },
        }
    }
}