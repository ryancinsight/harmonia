//! Transactional two-partition Jacobi coupling.

mod algorithm;
mod error;
mod model;
mod report;
mod workspace;

pub use algorithm::PartitionedPair;
pub use error::{CouplingError, PairConstructionError, WorkspaceError};
pub use model::{PairComponents, PairModel};
pub use report::CouplingReport;
pub use workspace::PairWorkspace;
