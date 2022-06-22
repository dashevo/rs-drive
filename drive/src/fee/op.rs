use crate::drive::flags::StorageFlags;
use crate::fee::op::DriveOperation::{
    CalculatedCostOperation, ContractFetch, CostCalculationDeleteOperation,
    CostCalculationInsertOperation, CostCalculationQueryOperation, GroveOperation,
};
use costs::OperationCost;
use enum_map::{enum_map, Enum, EnumMap};
use grovedb::{batch::GroveDbOp, Element, GroveDb, PathQuery};

pub(crate) const STORAGE_CREDIT_PER_BYTE: u64 = 27000;
pub(crate) const STORAGE_PROCESSING_CREDIT_PER_BYTE: u64 = 10;
pub(crate) const QUERY_CREDIT_PER_BYTE: u64 = 10;
pub(crate) const STORAGE_SEEK_COST: u64 = 100;

#[derive(Debug, Enum)]
pub enum BaseOp {
    Stop,
    Add,
    Mul,
    Sub,
    Div,
    Sdiv,
    Mod,
    Smod,
    Addmod,
    Mulmod,
    Signextend,
    Lt,
    Gt,
    Slt,
    Sgt,
    Eq,
    Iszero,
    And,
    Or,
    Xor,
    Not,
    Byte,
}

impl BaseOp {
    pub fn cost(&self) -> u64 {
        match self {
            BaseOp::Stop => 0,
            BaseOp::Add => 12,
            BaseOp::Mul => 20,
            BaseOp::Sub => 12,
            BaseOp::Div => 20,
            BaseOp::Sdiv => 20,
            BaseOp::Mod => 20,
            BaseOp::Smod => 20,
            BaseOp::Addmod => 32,
            BaseOp::Mulmod => 32,
            BaseOp::Signextend => 20,
            BaseOp::Lt => 12,
            BaseOp::Gt => 12,
            BaseOp::Slt => 12,
            BaseOp::Sgt => 12,
            BaseOp::Eq => 12,
            BaseOp::Iszero => 12,
            BaseOp::And => 12,
            BaseOp::Or => 12,
            BaseOp::Xor => 12,
            BaseOp::Not => 12,
            BaseOp::Byte => 12,
        }
    }
}

#[derive(Debug, Enum)]
pub enum FunctionOp {
    Exp,
    Sha256,
    Sha256_2,
    Blake3,
}

impl FunctionOp {
    pub fn cost(&self, word_count: u32) {}
}

#[derive(Debug)]
pub struct SizesOfQueryOperation {
    pub key_size: u32,
    pub path_size: u32,
    pub value_size: u32,
}

impl SizesOfQueryOperation {
    pub fn for_key_check_in_path<'a: 'b, 'b, 'c, P>(key_len: usize, path: P) -> Self
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_size: u32 = path
            .into_iter()
            .map(|inner: &[u8]| inner.len() as u32)
            .sum();
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size,
            value_size: 0,
        }
    }

    pub fn for_key_check_with_path_length(key_len: usize, path_len: usize) -> Self {
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size: path_len as u32,
            value_size: 0,
        }
    }

    pub fn for_value_retrieval_in_path<'a: 'b, 'b, 'c, P>(
        key_len: usize,
        path: P,
        value_len: usize,
    ) -> Self
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_size: u32 = path
            .into_iter()
            .map(|inner: &[u8]| inner.len() as u32)
            .sum();
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size,
            value_size: value_len as u32,
        }
    }

    pub fn for_value_retrieval_with_path_length(
        key_len: usize,
        path_len: usize,
        value_len: usize,
    ) -> Self {
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size: path_len as u32,
            value_size: value_len as u32,
        }
    }

    pub fn for_path_query(path_query: &PathQuery, returned_values: &[Vec<u8>]) -> Self {
        SizesOfQueryOperation {
            key_size: path_query
                .query
                .query
                .items
                .iter()
                .map(|query_item| query_item.processing_footprint())
                .sum(),
            path_size: path_query.path.len() as u32,
            value_size: returned_values.iter().map(|v| v.len() as u32).sum(),
        }
    }

    pub fn for_empty_path_query(path_query: &PathQuery) -> Self {
        SizesOfQueryOperation {
            key_size: path_query
                .query
                .query
                .items
                .iter()
                .map(|query_item| query_item.processing_footprint())
                .sum(),
            path_size: path_query.path.len() as u32,
            value_size: 0,
        }
    }

    pub fn data_size(&self) -> u32 {
        self.path_size + self.key_size + self.value_size as u32
    }

    pub fn ephemeral_cost(&self) -> u64 {
        self.data_size() as u64 * QUERY_CREDIT_PER_BYTE
    }
}

#[derive(Debug)]
pub struct SizesOfInsertOperation {
    pub path_size: u32,
    pub key_size: u16,
    pub value_size: u32,
}

#[derive(Debug)]
pub struct SizesOfDeleteOperation {
    pub path_size: u32,
    pub key_size: u16,
    pub value_size: u32,
    pub multiplier: u8,
}

impl SizesOfDeleteOperation {
    pub fn for_empty_tree(path_size: u32, key_size: u16, multiplier: u8) -> Self {
        SizesOfDeleteOperation {
            path_size,
            key_size,
            value_size: 0,
            multiplier,
        }
    }
    pub fn for_key_value(path_size: u32, key_size: u16, element: &Element, multiplier: u8) -> Self {
        let value_size = match element {
            Element::Item(item, _) => item.len(),
            Element::Reference(path, _) => path.iter().map(|inner| inner.len()).sum(),
            Element::Tree(..) => 32,
        } as u32;
        SizesOfDeleteOperation::for_key_value_size(path_size, key_size, value_size, multiplier)
    }

    pub fn for_key_value_size(
        path_size: u32,
        key_size: u16,
        value_size: u32,
        multiplier: u8,
    ) -> Self {
        SizesOfDeleteOperation {
            path_size,
            key_size,
            value_size,
            multiplier,
        }
    }

    pub fn data_size(&self) -> u32 {
        self.value_size + self.key_size as u32
    }

    pub fn ephemeral_cost(&self) -> u64 {
        self.data_size() as u64 * STORAGE_PROCESSING_CREDIT_PER_BYTE
    }

    pub fn storage_cost(&self) -> i64 {
        -(self.data_size() as i64 * STORAGE_CREDIT_PER_BYTE as i64)
    }
}

#[derive(Debug)]
pub enum DriveOperation {
    GroveOperation(GroveDbOp),
    CalculatedCostOperation(OperationCost),
    CostCalculationInsertOperation(SizesOfInsertOperation),
    CostCalculationDeleteOperation(SizesOfDeleteOperation),
    CostCalculationQueryOperation(SizesOfQueryOperation),
    ContractFetch,
}

impl DriveOperation {
    pub fn grovedb_operations(insert_operations: &Vec<DriveOperation>) -> Vec<GroveDbOp> {
        insert_operations
            .iter()
            .filter_map(|op| match op {
                GroveOperation(grovedb_op) => Some(grovedb_op.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn for_empty_tree(path: Vec<Vec<u8>>, key: Vec<u8>, storage_flags: &StorageFlags) -> Self {
        let tree = Element::empty_tree_with_flags(storage_flags.to_element_flags());
        DriveOperation::for_path_key_element(path, key, tree)
    }
    pub fn for_path_key_element(path: Vec<Vec<u8>>, key: Vec<u8>, element: Element) -> Self {
        GroveOperation(GroveDbOp::insert(path, key, element))
    }

    pub fn for_insert_path_key_value_size(path_size: u32, key_size: u16, value_size: u32) -> Self {
        CostCalculationInsertOperation(SizesOfInsertOperation {
            path_size,
            key_size,
            value_size,
        })
    }

    pub fn for_delete_path_key_value_size(
        path: Vec<Vec<u8>>,
        key_size: u16,
        value_size: u32,
        multiplier: u8,
    ) -> Self {
        let path_sizes: Vec<u16> = path.into_iter().map(|x| x.len() as u16).collect();
        Self::for_delete_path_key_value_max_sizes(path_sizes, key_size, value_size, multiplier)
    }

    pub fn for_delete_path_key_value_max_sizes(
        path: Vec<u16>,
        key_size: u16,
        value_size: u32,
        multiplier: u8,
    ) -> Self {
        let path_size: u32 = path.into_iter().map(|x| x as u32).sum();
        CostCalculationDeleteOperation(SizesOfDeleteOperation::for_key_value_size(
            path_size, key_size, value_size, multiplier,
        ))
    }

    pub fn for_query_path_key_value_size(path_size: u32, key_size: u32, value_size: u32) -> Self {
        CostCalculationQueryOperation(SizesOfQueryOperation {
            path_size,
            key_size,
            value_size,
        })
    }

    pub fn data_size(&self) -> u32 {
        match self {
            GroveOperation(grovedb_op) => grovedb_op.key.len() as u32,
            CostCalculationInsertOperation(worst_case_insert_operation) => {
                let node_value_size = Element::calculate_node_byte_size(
                    worst_case_insert_operation.value_size as usize,
                    worst_case_insert_operation.key_size as usize,
                );
                node_value_size as u32
            }
            CostCalculationQueryOperation(worst_case_query_operation) => {
                worst_case_query_operation.data_size()
            }
            CostCalculationDeleteOperation(worst_case_delete_operation) => {
                worst_case_delete_operation.data_size()
            }
            CalculatedCostOperation(operation_cost) => operation_cost.storage_written_bytes as u32,
            ContractFetch => 0,
        }
    }

    pub fn ephemeral_cost(&self) -> u64 {
        self.data_size() as u64 * STORAGE_PROCESSING_CREDIT_PER_BYTE
    }

    pub fn storage_cost(&self) -> i64 {
        self.data_size() as i64 * STORAGE_CREDIT_PER_BYTE as i64
    }
}
