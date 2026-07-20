use eunomia::NumericElement;

use super::{Relaxation, RelaxationError};

/// Zero-sized full fixed-point update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FullRelaxation;

impl<T> Relaxation<T> for FullRelaxation
where
    T: NumericElement,
{
    fn update(&self, current: &mut [T], candidate: &[T]) -> Result<(), RelaxationError> {
        if current.len() != candidate.len() {
            return Err(RelaxationError::Dimension {
                current: current.len(),
                candidate: candidate.len(),
            });
        }
        for (index, (destination, source)) in current
            .iter_mut()
            .zip(candidate.iter().copied())
            .enumerate()
        {
            if !source.is_finite() {
                return Err(RelaxationError::NonFinite { index });
            }
            *destination = source;
        }
        Ok(())
    }
}
