use eunomia::{NumericElement, RealField};

use super::{InvalidRelaxation, Relaxation, RelaxationError, slice::update_pair_slices};

/// Validated fixed under-relaxation policy.
///
/// A unit weight returns the candidate exactly. For other valid weights,
/// finite endpoints remain usable even if their direct difference overflows.
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
        if weight == <T as NumericElement>::ONE {
            return update_pair_slices(
                first_current,
                first_candidate,
                second_current,
                second_candidate,
                |current, candidate| {
                    if current.is_finite() {
                        candidate
                    } else {
                        current
                    }
                },
            );
        }
        update_pair_slices(
            first_current,
            first_candidate,
            second_current,
            second_candidate,
            |current, candidate| fixed_step(current, candidate, weight),
        )
    }
}

/// Retain the rounding residual of a finite difference before interpolation.
#[inline]
fn fixed_step<T: RealField>(current: T, candidate: T, weight: T) -> T {
    let difference = candidate - current;
    if current.is_finite() && candidate.is_finite() {
        if !difference.is_finite() {
            return overflow_safe_step(current, candidate, weight);
        }
        let (_, difference_error) = two_sum(candidate, -current);
        let update = weight.scalar_fmadd(difference, current);
        if difference_error != <T as NumericElement>::ZERO {
            return weight.scalar_fmadd(difference_error, update);
        }
        return update;
    }
    weight.scalar_fmadd(difference, current)
}

/// Evaluate the convex update when opposite finite endpoints overflow their
/// direct difference. Orient the segment so its fraction is at most one half;
/// the subtraction from one is exact for weights above one half. Decompose
/// the two products and two sums so cancellation retains their low parts.
#[inline]
fn overflow_safe_step<T: RealField>(current: T, candidate: T, weight: T) -> T {
    let half = T::from_f64(0.5);
    let (start, end, fraction) = if weight > half {
        (candidate, current, <T as NumericElement>::ONE - weight)
    } else {
        (current, candidate, weight)
    };

    let start_product = fraction * start;
    let start_product_error = fraction.scalar_fmadd(start, -start_product);
    let (base, base_error) = two_sum(start, -start_product);
    let end_product = fraction * end;
    let end_product_error = fraction.scalar_fmadd(end, -end_product);
    let (sum, sum_error) = two_sum(base, end_product);
    sum_expansion([
        base_error,
        -start_product_error,
        sum_error,
        end_product_error,
        sum,
    ])
}

/// Error-free sum decomposition when the sum remains finite.
#[inline]
fn two_sum<T: RealField>(left: T, right: T) -> (T, T) {
    let sum = left + right;
    let virtual_right = sum - left;
    let error = (left - (sum - virtual_right)) + (right - virtual_right);
    (sum, error)
}

/// Sum the short non-overflowing expansion from least to most significant,
/// correcting a final halfway case when the remaining tail has the same sign.
fn sum_expansion<T: RealField>(terms: [T; 5]) -> T {
    let zero = <T as NumericElement>::ZERO;
    let mut partials = [zero; 5];
    let mut count = 0;
    for mut value in terms {
        let mut next = 0;
        let mut index = 0;
        while index < count {
            let mut part = partials[index];
            if value.abs() < part.abs() {
                core::mem::swap(&mut value, &mut part);
            }
            let high = value + part;
            let low = part - (high - value);
            if low != zero {
                partials[next] = low;
                next += 1;
            }
            value = high;
            index += 1;
        }
        partials[next] = value;
        count = next + 1;
    }

    count -= 1;
    let mut high = partials[count];
    let mut low = zero;
    while count > 0 {
        count -= 1;
        let previous = high;
        let part = partials[count];
        high = previous + part;
        low = part - (high - previous);
        if low != zero {
            break;
        }
    }
    if count > 0
        && ((low < zero && partials[count - 1] < zero)
            || (low > zero && partials[count - 1] > zero))
    {
        let twice_low = low + low;
        let adjusted = high + twice_low;
        if adjusted - high == twice_low {
            high = adjusted;
        }
    }
    high
}
