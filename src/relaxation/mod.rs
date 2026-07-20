//! Fixed-point relaxation contracts and policies.

mod contract;
mod error;
mod fixed;
mod full;

pub use contract::Relaxation;
pub use error::{InvalidRelaxation, RelaxationError};
pub use fixed::FixedRelaxation;
pub use full::FullRelaxation;
