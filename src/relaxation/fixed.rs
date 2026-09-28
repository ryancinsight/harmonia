use eunomia::{NumericElement, RealField};

use super::{InvalidRelaxation, Relaxation, RelaxationError, slice::update_pair_slices};

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
        let weight = self.weight;
        update_pair_slices(
            first_current,
            first_candidate,
            second_current,
            second_candidate,
            |current, candidate| weight.scalar_fmadd(candidate - current, current),
        )
    }
}
