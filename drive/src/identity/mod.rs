use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use grovedb::Error;
use crate::drive::Drive;
use ciborium::value::{Value as CborValue};
use crate::common;
use crate::common::bytes_for_system_value_from_hash_map;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Identity {
    pub id: [u8; 32],
    pub revision: u64,
    pub balance: u64,
    pub keys: BTreeMap<u16, IdentityKey>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct IdentityKey {
    pub id: u16,
    pub key_type: u8,
    pub public_key_bytes: Vec<u8>,
    pub purpose: u8,
    pub security_level: u8,
    pub readonly: bool,
}

impl IdentityKey {
    pub fn from_cbor_value(
        key_value_map: &[(CborValue, CborValue)],
    ) -> Result<Self, Error> {

        let id = match common::cbor_inner_u16_value(key_value_map, "id") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::CorruptedData(String::from(
                    "a key must have an id",
                )))
            }
        };

        let key_type = match common::cbor_inner_u8_value(key_value_map, "type") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::CorruptedData(String::from(
                    "a key must have a type",
                )))
            }
        };

        let purpose = match common::cbor_inner_u8_value(key_value_map, "purpose") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::CorruptedData(String::from(
                    "a key must have a purpose",
                )))
            }
        };

        let security_level = match common::cbor_inner_u8_value(key_value_map, "securityLevel") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::CorruptedData(String::from(
                    "a key must have a securityLevel",
                )))
            }
        };

        let readonly = match common::cbor_inner_bool_value(key_value_map, "readOnly") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::CorruptedData(String::from(
                    "a key must have a readOnly value",
                )))
            }
        };

        let public_key_bytes = match common::cbor_inner_array_value(key_value_map, "data") {
            Some(index_values) => index_values,
            None => {
                return Err(Error::CorruptedData(String::from(
                    "a key must have a readOnly value",
                )))
            }
        };

        Ok(IdentityKey{
            id,
            key_type,
            public_key_bytes: vec![],
            purpose,
            security_level,
            readonly,
        })
    }
}

impl Identity {
    pub fn from_cbor(identity_cbor: &[u8]) -> Result<Self, Error> {
        let (version, read_identity_cbor) = identity_cbor.split_at(4);
        if !Drive::check_protocol_version_bytes(version) {
            return Err(Error::CorruptedData(String::from(
                "invalid protocol version",
            )));
        }
        // Deserialize the contract
        let identity: HashMap<String, CborValue> = ciborium::de::from_reader(read_identity_cbor)
            .map_err(|e| {
                Error::CorruptedData(String::from("unable to decode identity"))
            })?;

        // Get the contract id
        let identity_id: [u8; 32] = bytes_for_system_value_from_hash_map(&identity, "id")
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get identity id")))?
            .try_into()
            .map_err(|_| Error::CorruptedData(String::from("id must be 32 bytes")))?;

        let revision : u64 = identity.get("revision")
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get revision")))?
            .as_integer()
            .ok_or_else(|| Error::CorruptedData(String::from("revision must be an integer")))?
            .try_into()
            .map_err(|_| Error::CorruptedData(String::from("revision must be in the range of a unsigned 64 bit integer")))?;

        let balance : u64 = identity.get("balance")
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get revision")))?
            .as_integer()
            .ok_or_else(|| Error::CorruptedData(String::from("revision must be an integer")))?
            .try_into()
            .map_err(|_| Error::CorruptedData(String::from("revision must be in the range of a unsigned 64 bit integer")))?;

        let keys_cbor_value = identity
            .get("publicKeys")
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get keys")))?;
        let keys_cbor_value_raw = keys_cbor_value
            .as_array()
            .ok_or_else(|| Error::CorruptedData(String::from("unable to get keys as map")))?;

        let mut keys: BTreeMap<u16, IdentityKey> = BTreeMap::new();

        // Build the document type hashmap
        for key in keys_cbor_value_raw {
            match key.as_map() {
                None => {
                    return Err(Error::CorruptedData(String::from(
                        "key value is not a map as expected",
                    )));
                }
                Some(map) => {
                    let key = IdentityKey::from_cbor_value(map)?;
                    keys.insert(key.id, key);
                }
            }
        }

        Ok(Identity {
            id: identity_id,
            revision,
            balance,
            keys,
        })
    }
}