use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

use grovedb::{GroveDb, Transaction, TransactionArg};
use moka::sync::Cache;

use object_size_info::DocumentAndContractInfo;
use object_size_info::DocumentInfo::DocumentSize;

use crate::contract::Contract;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::config::DriveConfig;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::op::DriveOperation::GroveOperation;

pub mod batch;
pub mod config;
pub mod contract;
pub mod defaults;
pub mod document;
pub mod fee_pools;
pub mod flags;
pub mod genesis_time;
mod grove_operations;
pub mod identity;
pub mod initialization;
pub mod object_size_info;
pub mod query;

use dpp::data_contract::extra::DriveContractExt;

pub struct DriveCache {
    pub cached_contracts: Cache<[u8; 32], Arc<Contract>>,
    pub genesis_time_ms: Option<u64>,
}

pub struct Drive {
    pub grove: GroveDb,
    pub config: DriveConfig,
    pub cache: RefCell<DriveCache>,
}

#[repr(u8)]
pub enum RootTree {
    // Input data errors
    Identities = 0,
    ContractDocuments = 1,
    PublicKeyHashesToIdentities = 2,
    SpentAssetLockTransactions = 3,
    Pools = 4,
}

pub const STORAGE_COST: i32 = 50;

impl From<RootTree> for u8 {
    fn from(root_tree: RootTree) -> Self {
        root_tree as u8
    }
}

impl From<RootTree> for [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        [root_tree as u8]
    }
}

impl From<RootTree> for &'static [u8; 1] {
    fn from(root_tree: RootTree) -> Self {
        match root_tree {
            RootTree::Identities => &[0],
            RootTree::ContractDocuments => &[1],
            RootTree::PublicKeyHashesToIdentities => &[2],
            RootTree::SpentAssetLockTransactions => &[3],
            RootTree::Pools => &[4],
        }
    }
}

fn contract_documents_path(contract_id: &[u8]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
        contract_id,
        &[1],
    ]
}

impl Drive {
    pub fn open<P: AsRef<Path>>(path: P, config: Option<DriveConfig>) -> Result<Self, Error> {
        match GroveDb::open(path) {
            Ok(grove) => {
                let config = config.unwrap_or_default();
                let genesis_time_ms = config.default_genesis_time.clone();
                Ok(Drive {
                    grove,
                    config,
                    cache: RefCell::new(DriveCache {
                        cached_contracts: Cache::new(200),
                        genesis_time_ms,
                    }),
                })
            }
            Err(e) => Err(Error::GroveDB(e)),
        }
    }

    pub fn commit_transaction(&self, transaction: Transaction) -> Result<(), Error> {
        self.grove
            .commit_transaction(transaction)
            .unwrap() // TODO: discuss what to do with transaction cost as costs are
            // returned in advance on transaction operations not on commit
            .map_err(Error::GroveDB)
    }

    pub fn rollback_transaction(&self, transaction: &Transaction) -> Result<(), Error> {
        self.grove
            .rollback_transaction(transaction)
            .map_err(Error::GroveDB)
    }

    pub const fn check_protocol_version(_version: u32) -> bool {
        // Temporary disabled due protocol version is dynamic and goes from consensus params
        true
    }

    pub fn check_protocol_version_bytes(version_bytes: &[u8]) -> bool {
        if version_bytes.len() != 4 {
            false
        } else {
            let version_set_bytes: [u8; 4] = version_bytes
                .try_into()
                .expect("slice with incorrect length");
            let version = u32::from_be_bytes(version_set_bytes);
            Drive::check_protocol_version(version)
        }
    }

    fn apply_batch_drive_operations(
        &self,
        apply: bool,
        transaction: TransactionArg,
        batch_operations: Vec<DriveOperation>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let grove_db_operations = DriveOperation::grovedb_operations_batch(&batch_operations);
        self.apply_batch_grovedb_operations(
            apply,
            transaction,
            grove_db_operations,
            drive_operations,
        )?;
        batch_operations.into_iter().for_each(|op| match op {
            GroveOperation(_) => (),
            _ => drive_operations.push(op),
        });
        Ok(())
    }

    fn apply_batch_grovedb_operations(
        &self,
        apply: bool,
        transaction: TransactionArg,
        batch_operations: GroveDbOpBatch,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if apply {
            self.grove_apply_batch_with_add_costs(
                batch_operations,
                false,
                transaction,
                drive_operations,
            )?;
        } else {
            self.grove_batch_operations_costs(batch_operations, false, drive_operations)?;
        }
        Ok(())
    }

    pub fn worst_case_fee_for_document_type_with_name(
        &self,
        contract: &Contract,
        document_type_name: &str,
    ) -> Result<(i64, u64), Error> {
        let document_type = contract.document_type_for_name(document_type_name)?;
        self.add_document_for_contract(
            DocumentAndContractInfo {
                document_info: DocumentSize(document_type.max_size()),
                contract,
                document_type,
                owner_id: None,
            },
            false,
            0.0,
            false,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::option::Option::None;

    use tempfile::TempDir;

    use crate::common::json_document_to_cbor;
    use crate::drive::Drive;

    #[test]
    fn store_document_1() {
        let tmp_dir = TempDir::new().unwrap();
        let _drive = Drive::open(tmp_dir, None);
    }

    #[test]
    fn test_cbor_deserialization() {
        let serialized_document = json_document_to_cbor("simple.json", Some(1));
        let (version, read_serialized_document) = serialized_document.split_at(4);
        assert!(Drive::check_protocol_version_bytes(version));
        let document: HashMap<String, ciborium::value::Value> =
            ciborium::de::from_reader(read_serialized_document).expect("cannot deserialize cbor");
        assert!(document.get("a").is_some());
        let tmp_dir = TempDir::new().unwrap();
        let _drive = Drive::open(tmp_dir, None);
    }
}
