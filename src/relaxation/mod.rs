//! Fixed-point relaxation contracts and policies.

mod aitken;
mod contract;
mod error;
mod fixed;
mod full;
mod slice;

pub use aitken::AitkenRelaxation;
pub use contract::Relaxation;
pub use error::{InvalidAitkenRelaxation, InvalidRelaxation, RelaxationError};
pub use fixed::FixedRelaxation;
pub use full::FullRelaxation;
