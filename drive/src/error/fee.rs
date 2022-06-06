#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    #[error("requesting worst case delete fee for known item error: {0}")]
    RequestingWorstCaseDeleteFeeForKnownItem(&'static str),
}
