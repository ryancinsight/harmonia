use alloc::borrow::Cow;

use super::{Transfer, TransferError};

/// Const-generic single-entry selection transfer.
///
/// `SOURCE` selects one source entry. The index is stored in the type, so the
/// policy itself is zero-sized and each selection monomorphizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndexTransfer<const SOURCE: usize>;

impl<T, const SOURCE: usize> Transfer<T> for IndexTransfer<SOURCE>
where
    T: Clone,
{
    #[inline]
    fn destination_dimension(&self, _source_dimension: usize) -> usize {
        1
    }

    fn transfer<'a>(
        &self,
        source: &'a [T],
        scratch: &'a mut [T],
    ) -> Result<Cow<'a, [T]>, TransferError> {
        if scratch.len() != 1 {
            return Err(TransferError::Dimension {
                expected: 1,
                actual: scratch.len(),
            });
        }
        let Some(value) = source.get(SOURCE) else {
            return Err(TransferError::SourceIndex {
                index: SOURCE,
                source_dimension: source.len(),
            });
        };
        scratch[0].clone_from(value);
        Ok(Cow::Borrowed(scratch))
    }
}
