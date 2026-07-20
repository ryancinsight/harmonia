use eunomia::{NumericElement, RealField};

use super::{InvalidRelaxation, Relaxation, RelaxationError};

/// Validated fixed under-relaxation policy.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedRelaxation<T> {
    weight: T,
}

impl<T> FixedRelaxation<T>
where
    T: RealField,
{
    /// Construct a weight in `(0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidRelaxation::OutsideUnitInterval`] for a non-finite,
    /// non-positive, or greater-than-one weight.
    pub fn new(weight: T) -> Result<Self, InvalidRelaxation> {
        if !weight.is_finite()
            || weight <= <T as NumericElement>::ZERO
            || weight > <T as NumericElement>::ONE
        {
            return Err(InvalidRelaxation::OutsideUnitInterval);
        }
        Ok(Self { weight })
    }

    /// Relaxation weight.
    #[inline]
    #[must_use]
    pub const fn weight(&self) -> T {
        self.weight
    }
}

impl<T> Relaxation<T> for FixedRelaxation<T>
where
    T: RealField,
{
    fn update(&self, current: &mut [T], candidate: &[T]) -> Result<(), RelaxationError> {
        if current.len() != candidate.len() {
            return Err(RelaxationError::Dimension {
                current: current.len(),
                candidate: candidate.len(),
            });
        }
        for (index, (value, target)) in current
            .iter_mut()
            .zip(candidate.iter().copied())
            .enumerate()
        {
            let updated = self.weight.scalar_fmadd(target - *value, *value);
            if !updated.is_finite() {
                return Err(RelaxationError::NonFinite { index });
            }
            *value = updated;
        }
        Ok(())
    }
}
