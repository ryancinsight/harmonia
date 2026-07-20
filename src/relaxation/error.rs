use core::fmt;

/// Invalid fixed relaxation weight.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidRelaxation {
    /// Weight is non-finite, non-positive, or greater than one.
    OutsideUnitInterval,
}

impl fmt::Display for InvalidRelaxation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("relaxation weight must be finite and in (0, 1]")
    }
}

impl core::error::Error for InvalidRelaxation {}

/// Relaxation update failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelaxationError {
    /// Current and candidate slices differ in length.
    Dimension {
        /// Current entries.
        current: usize,
        /// Candidate entries.
        candidate: usize,
    },
    /// An updated entry became non-finite.
    NonFinite {
        /// Invalid entry index.
        index: usize,
    },
}

impl fmt::Display for RelaxationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Dimension { current, candidate } => write!(
                formatter,
                "relaxation dimension mismatch: current {current}, candidate {candidate}"
            ),
            Self::NonFinite { index } => {
                write!(formatter, "relaxed interface entry {index} is non-finite")
            }
        }
    }
}

impl core::error::Error for RelaxationError {}
