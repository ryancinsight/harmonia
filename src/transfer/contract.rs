use alloc::borrow::Cow;

use super::TransferError;

/// Borrow-preserving interface transfer.
///
/// A transfer may return the source slice directly or populate and borrow the
/// caller-owned scratch slice. The coupling loop therefore needs no implicit
/// allocation.
pub trait Transfer<T>
where
    T: Clone,
{
    /// Required destination dimension for `source_dimension`.
    fn destination_dimension(&self, source_dimension: usize) -> usize;

    /// Transfer `source` into a borrowed result.
    ///
    /// # Errors
    ///
    /// Returns a dimensional or value failure without allocating.
    fn transfer<'a>(
        &self,
        source: &'a [T],
        scratch: &'a mut [T],
    ) -> Result<Cow<'a, [T]>, TransferError>;
}
