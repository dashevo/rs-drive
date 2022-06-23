use std::collections::BTreeMap;

use ciborium::value::Value;
use grovedb::TransactionArg;

use crate::{
    contract::{Contract, Document},
    drive::{
        flags::StorageFlags,
        object_size_info::{DocumentAndContractInfo, DocumentInfo::DocumentAndSerialization},
        Drive,
    },
    fee::pools::constants,
    identity::Identity,
};

fn create_mn_shares_contract(drive: &Drive) -> Contract {
    let contract_hex = "01000000a56324696458200cace205246693a7c8156523620daa937d2f2247934463eeb01ff7219590958c6724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e65724964582024da2bb09da5b1429f717ac1ce6537126cc65215f1d017e67b65eb252ef964b76776657273696f6e0169646f63756d656e7473a16b7265776172645368617265a66474797065666f626a65637467696e646963657382a3646e616d65716f776e65724964416e64506179546f496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a167706179546f496463617363a2646e616d65676f776e657249646a70726f7065727469657381a168246f776e65724964636173636872657175697265648267706179546f49646a70657263656e746167656a70726f70657274696573a267706179546f4964a66474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e781f4964656e74696669657220746f20736861726520726577617264207769746870636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726a70657263656e74616765a4647479706567696e7465676572676d6178696d756d192710676d696e696d756d016b6465736372697074696f6e781a5265776172642070657263656e7461676520746f2073686172656b6465736372697074696f6e78405368617265207370656369666965642070657263656e74616765206f66206d61737465726e6f646520726577617264732077697468206964656e746974696573746164646974696f6e616c50726f70657274696573f4";

    let contract_cbor = hex::decode(contract_hex).expect("Decoding failed");

    let contract =
        Contract::from_cbor(&contract_cbor, None).expect("expected to deserialize the contract");

    drive
        .apply_contract(
            &contract,
            contract_cbor.clone(),
            0f64,
            true,
            StorageFlags { epoch: 0 },
            None,
        )
        .expect("expected to apply contract successfully");

    contract
}

fn create_identity(id: [u8; 32], drive: &Drive, transaction: TransactionArg) -> Identity {
    let identity = Identity {
        id,
        revision: 1,
        balance: 0,
        keys: BTreeMap::new(),
    };

    drive
        .insert_identity_cbor(Some(&identity.id), identity.to_cbor(), true, transaction)
        .expect("to insert identity");

    identity
}

fn create_mn_identity(
    pro_tx_hash: [u8; 32],
    drive: &Drive,
    transaction: TransactionArg,
) -> Identity {
    create_identity(
        bs58::encode(pro_tx_hash)
            .into_vec()
            .try_into()
            .expect("id to be 32 bytes long"),
        &drive,
        transaction,
    )
}

fn create_mn_share_document(
    contract: &Contract,
    identity: &Identity,
    payToIdentity: &Identity,
    percentage: u16,
    drive: &Drive,
) -> Document {
    let id = rand::random::<[u8; 32]>();

    let properties: BTreeMap<String, Value> = BTreeMap::new();

    properties.insert(String::from("payToId"), Value::Bytes(identity.id.to_vec()));
    properties.insert(String::from("percentage"), percentage.into());

    let document = Document {
        id,
        properties,
        owner_id: identity.id,
    };

    let document_type = contract
        .document_type_for_name(constants::MN_REWARD_SHARES_DOCUMENT_TYPE)
        .expect("expected to get a document type");

    let storage_flags = StorageFlags { epoch: 0 };

    let document_cbor = document.to_cbor();

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                document_info: DocumentAndSerialization((
                    &document,
                    &document_cbor,
                    &storage_flags,
                )),
                contract: &contract,
                document_type,
                owner_id: None,
            },
            false,
            0f64,
            true,
            None,
        )
        .expect("expected to insert a document successfully");

    document
}
