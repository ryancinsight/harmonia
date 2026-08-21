//! Typed physical fields exchanged at multiphysics boundaries.

mod envelope;
mod error;
mod geometry;

pub use envelope::FieldEnvelope;
pub use error::FieldError;
pub use geometry::GridGeometry;
