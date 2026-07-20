//! Coupling algorithm families.

mod pair;

pub use pair::{
    CouplingError, CouplingReport, PairComponents, PairConstructionError, PairModel, PairWorkspace,
    PartitionedPair, WorkspaceError,
};
