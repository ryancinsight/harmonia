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
    fn update_pair(
        &mut self,
        first_current: &mut [T],
        first_candidate: &[T],
        second_current: &mut [T],
        second_candidate: &[T],
    ) -> Result<(), RelaxationError> {
        Self::validate_slice(self.weight, first_current, first_candidate, 0)?;
        Self::validate_slice(
            self.weight,
            second_current,
            second_candidate,
            first_current.len(),
        )?;
        Self::apply_slice(self.weight, first_current, first_candidate);
        Self::apply_slice(self.weight, second_current, second_candidate);
        Ok(())
    }
}

impl<T> FixedRelaxation<T>
where
    T: RealField,
{
    fn validate_slice(
        weight: T,
        current: &mut [T],
        candidate: &[T],
        index_offset: usize,
    ) -> Result<(), RelaxationError> {
        if current.len() != candidate.len() {
            return Err(RelaxationError::Dimension {
                current: current.len(),
                candidate: candidate.len(),
            });
        }
        for (index, (value, target)) in current.iter().zip(candidate.iter().copied()).enumerate() {
            let updated = weight.scalar_fmadd(target - *value, *value);
            if !updated.is_finite() {
                return Err(RelaxationError::NonFinite {
                    index: index_offset + index,
                });
            }
        }
        Ok(())
    }

    fn apply_slice(weight: T, current: &mut [T], candidate: &[T]) {
        for (value, target) in current.iter_mut().zip(candidate.iter().copied()) {
            *value = weight.scalar_fmadd(target - *value, *value);
        }
    }
}
