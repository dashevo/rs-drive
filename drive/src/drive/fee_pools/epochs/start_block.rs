use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use grovedb::{Element, TransactionArg};

use crate::fee_pools::epochs::tree_key_constants;

impl Drive {
    pub fn get_epoch_start_block_height(
        &self,
        epoch_pool: &Epoch,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                epoch_pool.get_path(),
                tree_key_constants::KEY_START_BLOCK_HEIGHT.as_slice(),
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        if let Element::Item(item, _) = element {
            Ok(u64::from_be_bytes(item.as_slice().try_into().map_err(
                |_| Error::Fee(FeeError::CorruptedStartBlockHeightItemLength()),
            )?))
        } else {
            Err(Error::Fee(FeeError::CorruptedStartBlockHeightNotItem()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::tests::helpers::setup::{setup_drive, setup_fee_pools};
    use crate::fee_pools::epochs::tree_key_constants;
    use grovedb::Element;
    use rust_decimal_macros::dec;

    use crate::error;
    use crate::error::fee::FeeError;

    use super::Epoch;

    #[test]
    fn test_update_epoch_start_block_height() {
        let drive = setup_drive();

        let (transaction, _) = setup_fee_pools(&drive, None);

        let epoch = Epoch::new(0);

        let start_block_height = 1;

        let op = epoch.update_start_block_height_operation(start_block_height);

        drive
            .grove_apply_operation(op, false, Some(&transaction))
            .expect("should apply batch");

        let actual_start_block_height = drive
            .get_epoch_start_block_height(&epoch, Some(&transaction))
            .expect("should get start block height");

        assert_eq!(start_block_height, actual_start_block_height);
    }

    mod get_epoch_start_block_height {
        #[test]
        fn test_error_if_epoch_pool_is_not_initiated() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let non_initiated_epoch = super::Epoch::new(7000);

            match drive.get_epoch_start_block_height(&non_initiated_epoch, Some(&transaction)) {
                Ok(_) => assert!(
                    false,
                    "should not be able to get start block height on uninit epochs pool"
                ),
                Err(e) => match e {
                    super::error::Error::GroveDB(grovedb::Error::PathNotFound(_)) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_is_not_set() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::Epoch::new(0);

            match drive.get_epoch_start_block_height(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "must be an error"),
                Err(e) => match e {
                    super::error::Error::GroveDB(_) => assert!(true),
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_value_has_invalid_length() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::tree_key_constants::KEY_START_BLOCK_HEIGHT.as_slice(),
                    super::Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_start_block_height(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedStartBlockHeightItemLength(),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }

        #[test]
        fn test_error_if_element_has_invalid_type() {
            let drive = super::setup_drive();

            let (transaction, _) = super::setup_fee_pools(&drive, None);

            let epoch = super::Epoch::new(0);

            drive
                .grove
                .insert(
                    epoch.get_path(),
                    super::tree_key_constants::KEY_START_BLOCK_HEIGHT.as_slice(),
                    super::Element::empty_tree(),
                    Some(&transaction),
                )
                .unwrap()
                .expect("should insert invalid data");

            match drive.get_epoch_start_block_height(&epoch, Some(&transaction)) {
                Ok(_) => assert!(false, "should not be able to decode stored value"),
                Err(e) => match e {
                    super::error::Error::Fee(
                        super::FeeError::CorruptedStartBlockHeightNotItem(),
                    ) => {
                        assert!(true)
                    }
                    _ => assert!(false, "invalid error type"),
                },
            }
        }
    }
}
