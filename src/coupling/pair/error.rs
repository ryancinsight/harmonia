use core::fmt;

use horae::{subcycling::SubcycleError, time::TimeError};

use crate::{RelaxationError, TransferError};

/// Invalid pair-workspace construction.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceError {
    /// A partition reported a zero-sized state or interface.
    ZeroDimension {
        /// Dimension role.
        role: &'static str,
    },
    /// A transfer's destination does not match the receiving partition input.
    TransferDimension {
        /// Directed transfer role.
        role: &'static str,
        /// Transfer destination entries.
        transfer: usize,
        /// Receiving input entries.
        input: usize,
    },
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ZeroDimension { role } => {
                write!(formatter, "{role} dimension must be positive")
            }
            Self::TransferDimension {
                role,
                transfer,
                input,
            } => write!(
                formatter,
                "{role} destination dimension {transfer} does not match receiving input {input}"
            ),
        }
    }
}

impl core::error::Error for WorkspaceError {}

/// Pair-construction failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairConstructionError {
    /// Workspace dimensions are invalid.
    Workspace(WorkspaceError),
    /// A const-generic subcycle plan is invalid.
    Subcycle(SubcycleError),
}

impl fmt::Display for PairConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace(error) => write!(formatter, "invalid pair workspace: {error}"),
            Self::Subcycle(error) => write!(formatter, "invalid pair subcycle plan: {error}"),
        }
    }
}

impl core::error::Error for PairConstructionError {}

/// Transactional pair-coupling failure.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum CouplingError<T, FirstError, SecondError> {
    /// Caller slice does not match its validated workspace dimension.
    Dimension {
        /// Slice role.
        role: &'static str,
        /// Required entries.
        expected: usize,
        /// Supplied entries.
        actual: usize,
    },
    /// Typed time construction or advancement failed.
    Time(TimeError),
    /// Const-generic subcycle plan or child step is invalid.
    Subcycle(SubcycleError),
    /// Directed interface transfer failed.
    Transfer(TransferError),
    /// Interface relaxation failed.
    Relaxation(RelaxationError),
    /// First partition failed.
    First(FirstError),
    /// Second partition failed.
    Second(SecondError),
    /// A fixed-point metric became non-finite.
    NonFiniteMetric,
    /// Athena's iteration budget was exhausted without convergence.
    NotConverged {
        /// Completed fixed-point iterations.
        iterations: usize,
        /// Final checked raw defect.
        residual_norm: T,
        /// Effective Athena threshold.
        threshold: T,
    },
}

impl<T, FirstError, SecondError> fmt::Display for CouplingError<T, FirstError, SecondError>
where
    T: fmt::Display,
    FirstError: fmt::Display,
    SecondError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dimension {
                role,
                expected,
                actual,
            } => write!(
                formatter,
                "{role} dimension mismatch: expected {expected}, received {actual}"
            ),
            Self::Time(error) => write!(formatter, "coupling time failure: {error}"),
            Self::Subcycle(error) => write!(formatter, "coupling subcycle failure: {error}"),
            Self::Transfer(error) => write!(formatter, "coupling transfer failure: {error}"),
            Self::Relaxation(error) => write!(formatter, "coupling relaxation failure: {error}"),
            Self::First(error) => write!(formatter, "first partition failed: {error}"),
            Self::Second(error) => write!(formatter, "second partition failed: {error}"),
            Self::NonFiniteMetric => formatter.write_str("coupling metric became non-finite"),
            Self::NotConverged {
                iterations,
                residual_norm,
                threshold,
            } => write!(
                formatter,
                "coupling did not converge in {iterations} iterations: residual {residual_norm}, threshold {threshold}"
            ),
        }
    }
}

impl<T, FirstError, SecondError> core::error::Error for CouplingError<T, FirstError, SecondError>
where
    T: fmt::Debug + fmt::Display,
    FirstError: fmt::Debug + fmt::Display,
    SecondError: fmt::Debug + fmt::Display,
{
}
