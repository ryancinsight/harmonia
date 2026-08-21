//! Atlas partitioned multiphysics coupling orchestration.
//!
//! Harmonia owns coupling iteration, interface transfer, relaxation, and
//! transactional state exchange. Physics, time integration, and convergence
//! laws remain in their authoritative provider crates.

#![doc = include_str!("../README.md")]
#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;

/// Coupling algorithms and reusable workspaces.
pub mod coupling;
/// Quantity-typed, geometry-validated physical-field exchange.
pub mod field;
/// Partition contracts and typed substeps.
pub mod partition;
/// Fixed-point relaxation policies.
pub mod relaxation;
/// Borrow-preserving interface transfer policies.
pub mod transfer;

pub use coupling::{
    CouplingError, CouplingReport, PairComponents, PairConstructionError, PairModel, PairWorkspace,
    PartitionedPair, WorkspaceError,
};
pub use field::{FieldEnvelope, FieldError, GridGeometry};
pub use partition::{Partition, Substep};
pub use relaxation::{
    AitkenRelaxation, FixedRelaxation, FullRelaxation, InvalidAitkenRelaxation, InvalidRelaxation,
    Relaxation, RelaxationError,
};
pub use transfer::{IdentityTransfer, IndexTransfer, Transfer, TransferError};
