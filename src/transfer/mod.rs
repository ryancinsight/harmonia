//! Interface transfer contracts and static policies.

mod contract;
mod error;
mod identity;
mod index;

pub use contract::Transfer;
pub use error::TransferError;
pub use identity::IdentityTransfer;
pub use index::IndexTransfer;
