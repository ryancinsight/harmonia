use alloc::borrow::Cow;

use super::{Transfer, TransferError};

/// Zero-sized identity transfer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdentityTransfer;

impl<T> Transfer<T> for IdentityTransfer
where
    T: Clone,
{
    #[inline]
    fn destination_dimension(&self, source_dimension: usize) -> usize {
        source_dimension
    }

    #[inline]
    fn transfer<'a>(
        &self,
        source: &'a [T],
        scratch: &'a mut [T],
    ) -> Result<Cow<'a, [T]>, TransferError> {
        if scratch.len() != source.len() {
            return Err(TransferError::Dimension {
                expected: source.len(),
                actual: scratch.len(),
            });
        }
        Ok(Cow::Borrowed(source))
    }
}
