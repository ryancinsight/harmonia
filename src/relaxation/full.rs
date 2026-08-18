use eunomia::NumericElement;

use super::{Relaxation, RelaxationError};

/// Zero-sized full fixed-point update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FullRelaxation;

impl<T> Relaxation<T> for FullRelaxation
where
    T: NumericElement,
{
    fn update_pair(
        &mut self,
        first_current: &mut [T],
        first_candidate: &[T],
        second_current: &mut [T],
        second_candidate: &[T],
    ) -> Result<(), RelaxationError> {
        Self::validate_slice(first_current, first_candidate, 0)?;
        Self::validate_slice(second_current, second_candidate, first_current.len())?;
        Self::apply_slice(first_current, first_candidate);
        Self::apply_slice(second_current, second_candidate);
        Ok(())
    }
}

impl FullRelaxation {
    fn validate_slice<T>(
        current: &mut [T],
        candidate: &[T],
        index_offset: usize,
    ) -> Result<(), RelaxationError>
    where
        T: NumericElement,
    {
        if current.len() != candidate.len() {
            return Err(RelaxationError::Dimension {
                current: current.len(),
                candidate: candidate.len(),
            });
        }
        for (index, source) in candidate.iter().copied().enumerate() {
            if !source.is_finite() {
                return Err(RelaxationError::NonFinite {
                    index: index_offset + index,
                });
            }
        }
        Ok(())
    }

    fn apply_slice<T>(current: &mut [T], candidate: &[T])
    where
        T: NumericElement,
    {
        current.copy_from_slice(candidate);
    }
}
