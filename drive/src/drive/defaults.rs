pub const PROTOCOL_VERSION: u32 = 1;
pub const CONTRACT_DOCUMENTS_PATH_HEIGHT: u16 = 4;
pub const BASE_CONTRACT_ROOT_PATH_SIZE: usize = 33; // 1 + 32
pub const BASE_CONTRACT_KEEPING_HISTORY_STORAGE_PATH_SIZE: usize = 34; // 1 + 32 + 1
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_STORAGE_TIME_REFERENCE_PATH: usize = 75;
pub const BASE_CONTRACT_DOCUMENTS_KEEPING_HISTORY_PRIMARY_KEY_PATH_FOR_DOCUMENT_ID_SIZE: usize = 67; // 1 + 32 + 1 + 1 + 32, then we need to add document_type_name.len()
pub const BASE_CONTRACT_DOCUMENTS_PATH: usize = 34;
pub const BASE_CONTRACT_DOCUMENTS_PRIMARY_KEY_PATH: usize = 35;
pub const DEFAULT_HASH_SIZE: usize = 32;
pub const DEFAULT_FLOAT_SIZE: usize = 8;
pub const EMPTY_TREE_STORAGE_SIZE: usize = 33;
