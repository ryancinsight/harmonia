use core::fmt;

/// Interface transfer failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferError {
    /// Caller scratch does not match the transfer's destination dimension.
    Dimension {
        /// Required entries.
        expected: usize,
        /// Supplied entries.
        actual: usize,
    },
    /// A const-generic source index lies outside the source slice.
    SourceIndex {
        /// Invalid source index.
        index: usize,
        /// Source slice length.
        source_dimension: usize,
    },
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Dimension { expected, actual } => write!(
                formatter,
                "transfer destination dimension mismatch: expected {expected}, received {actual}"
            ),
            Self::SourceIndex {
                index,
                source_dimension,
            } => write!(
                formatter,
                "transfer source index {index} is outside dimension {source_dimension}"
            ),
        }
    }
}

impl core::error::Error for TransferError {}
