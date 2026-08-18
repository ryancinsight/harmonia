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

/// Invalid Aitken relaxation configuration.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidAitkenRelaxation {
    /// A lower or upper relaxation bound is non-finite.
    NonFiniteBound,
    /// The lower relaxation bound is not positive.
    NonPositiveMinimum,
    /// The upper relaxation bound is below the lower bound.
    MaximumBelowMinimum,
    /// The residual-denominator tolerance is non-finite.
    NonFiniteTolerance,
    /// The residual-denominator tolerance is not positive.
    NonPositiveTolerance,
}

impl fmt::Display for InvalidAitkenRelaxation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFiniteBound => "Aitken relaxation bounds must be finite",
            Self::NonPositiveMinimum => "Aitken minimum relaxation must be positive",
            Self::MaximumBelowMinimum => "Aitken maximum relaxation must not be below its minimum",
            Self::NonFiniteTolerance => "Aitken residual tolerance must be finite",
            Self::NonPositiveTolerance => "Aitken residual tolerance must be positive",
        };
        formatter.write_str(message)
    }
}

impl core::error::Error for InvalidAitkenRelaxation {}

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
    ///
    /// The index addresses the concatenation of the first and second
    /// interfaces, with the second interface beginning after the first
    /// interface's length.
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
