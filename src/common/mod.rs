use crate::contract::Contract;
use crate::drive::Drive;
use byteorder::{BigEndian, WriteBytesExt};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tempdir::TempDir;

pub fn setup(prefix: &str) -> Drive {
    // setup code
    let tmp_dir = TempDir::new(prefix).unwrap();
    let mut drive: Drive = Drive::open(tmp_dir).expect("expected to open Drive successfully");

    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    drive.grove.start_transaction();

    drive
        .create_root_tree(Some(&db_transaction))
        .expect("expected to create root tree successfully");

    drive.grove.commit_transaction(db_transaction);

    drive
}

pub fn setup_contract(prefix: &str, path: &str) -> (Drive, Contract) {
    let mut drive = setup(prefix);
    let contract_cbor = json_document_to_cbor(path, Some(crate::drive::defaults::PROTOCOL_VERSION));
    let contract = Contract::from_cbor(&contract_cbor).expect("contract should be deserialized");

    let storage = drive.grove.storage();
    let db_transaction = storage.transaction();
    drive.grove.start_transaction();

    drive.apply_contract(&contract_cbor, Some(&db_transaction));

    drive.grove.commit_transaction(db_transaction);

    (drive, contract)
}

pub fn json_document_to_cbor(path: impl AsRef<Path>, protocol_version: Option<u32>) -> Vec<u8> {
    let file = File::open(path).expect("file not found");
    let reader = BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).expect("expected a valid json");
    value_to_cbor(json, protocol_version)
}

pub fn value_to_cbor(value: serde_json::Value, protocol_version: Option<u32>) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    if let Some(protocol_version) = protocol_version {
        buffer.write_u32::<BigEndian>(protocol_version);
    }
    ciborium::ser::into_writer(&value, &mut buffer).expect("unable to serialize into cbor");
    buffer
}

pub fn text_file_strings(path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(path).expect("file not found");
    let reader = io::BufReader::new(file).lines();
    reader.into_iter().map(|a| a.unwrap()).collect()
}
