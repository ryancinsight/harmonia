use alloc::vec::Vec;

use eunomia::{NumericElement, RealField};

use super::{InvalidAitkenRelaxation, Relaxation, RelaxationError, slice::validate_dimensions};

/// Stateful componentwise Aitken \(\Delta^2\) relaxation for a coupled pair.
///
/// The policy stacks both interface residuals into one history vector. Each
/// window starts with unit relaxation. Later updates within that window use
/// the Irons--Tuck componentwise secant estimate
/// \(\omega_i=-\omega_{i,k-1}r_{i,k-1}/(r_{i,k}-r_{i,k-1})\), retaining the
/// previous factor when the residual difference is within the configured
/// tolerance. Every factor is clamped to the configured interval.
#[derive(Clone, Debug)]
pub struct AitkenRelaxation<T> {
    minimum: T,
    maximum: T,
    residual_tolerance: T,
    previous_residual: Vec<T>,
    previous_relaxation: Vec<T>,
    residual_workspace: Vec<T>,
    relaxation_workspace: Vec<T>,
}

impl<T> AitkenRelaxation<T>
where
    T: RealField,
{
    /// Construct a bounded Aitken policy.
    ///
    /// `minimum` and `maximum` bound every computed relaxation factor, and
    /// `residual_tolerance` determines when a component's secant denominator
    /// is treated as numerically indistinguishable from zero. All three
    /// values are interpreted in the native precision of `T`.
    ///
    /// # Errors
    ///
    /// Returns a typed configuration error when a bound or tolerance is
    /// non-finite, non-positive where required, or ordered incorrectly.
    pub fn new(
        minimum: T,
        maximum: T,
        residual_tolerance: T,
    ) -> Result<Self, InvalidAitkenRelaxation> {
        if !minimum.is_finite() || !maximum.is_finite() {
            return Err(InvalidAitkenRelaxation::NonFiniteBound);
        }
        if minimum <= <T as NumericElement>::ZERO {
            return Err(InvalidAitkenRelaxation::NonPositiveMinimum);
        }
        if maximum < minimum {
            return Err(InvalidAitkenRelaxation::MaximumBelowMinimum);
        }
        if !residual_tolerance.is_finite() {
            return Err(InvalidAitkenRelaxation::NonFiniteTolerance);
        }
        if residual_tolerance <= <T as NumericElement>::ZERO {
            return Err(InvalidAitkenRelaxation::NonPositiveTolerance);
        }

        Ok(Self {
            minimum,
            maximum,
            residual_tolerance,
            previous_residual: Vec::new(),
            previous_relaxation: Vec::new(),
            residual_workspace: Vec::new(),
            relaxation_workspace: Vec::new(),
        })
    }

    /// Lower bound applied to every computed relaxation factor.
    #[must_use]
    pub const fn minimum(&self) -> T {
        self.minimum
    }

    /// Upper bound applied to every computed relaxation factor.
    #[must_use]
    pub const fn maximum(&self) -> T {
        self.maximum
    }

    /// Small-denominator threshold for the residual secant estimate.
    #[must_use]
    pub const fn residual_tolerance(&self) -> T {
        self.residual_tolerance
    }

    fn update_slice(
        &mut self,
        current: &mut [T],
        candidate: &[T],
        index_offset: usize,
        history_available: bool,
    ) -> Result<(), RelaxationError> {
        for (index, (current_value, candidate_value)) in current
            .iter()
            .copied()
            .zip(candidate.iter().copied())
            .enumerate()
        {
            let absolute_index = index_offset + index;
            if !current_value.is_finite() {
                return Err(RelaxationError::NonFinite {
                    index: absolute_index,
                });
            }
            if !candidate_value.is_finite() {
                return Err(RelaxationError::NonFinite {
                    index: absolute_index,
                });
            }

            let residual = candidate_value - current_value;
            if !residual.is_finite() {
                return Err(RelaxationError::NonFinite {
                    index: absolute_index,
                });
            }
            self.residual_workspace[absolute_index] = residual;

            let factor = if history_available {
                self.factor_for(absolute_index, residual)?
            } else {
                self.clamp_factor(<T as NumericElement>::ONE)
            };
            self.relaxation_workspace[absolute_index] = factor;

            let updated = factor.scalar_fmadd(residual, current_value);
            if !updated.is_finite() {
                return Err(RelaxationError::NonFinite {
                    index: absolute_index,
                });
            }
        }
        Ok(())
    }

    fn factor_for(&self, index: usize, residual: T) -> Result<T, RelaxationError> {
        let previous_residual = self.previous_residual[index];
        let previous_factor = self.previous_relaxation[index];
        let difference = residual - previous_residual;
        let factor = if difference.abs() <= self.residual_tolerance {
            previous_factor
        } else {
            // An overflowing difference joins opposite-sign finite residuals.
            // Halving both is exact here and bounds their difference by MAX.
            // Its original magnitude exceeds every finite tolerance, so the
            // absolute-policy comparison above must precede this scaling.
            let (denominator, denominator_shift) = if difference.is_finite() {
                (difference, 0)
            } else {
                let half = T::from_f64(0.5);
                (residual * half - previous_residual * half, 1)
            };
            let raw = scaled_secant(
                previous_factor,
                previous_residual,
                denominator,
                denominator_shift,
            )
            .ok_or(RelaxationError::NonFinite { index })?;
            if !raw.is_finite() {
                return Err(RelaxationError::NonFinite { index });
            }
            self.clamp_factor(raw)
        };
        Ok(factor)
    }

    fn clamp_factor(&self, value: T) -> T {
        value.min_scalar(self.maximum).max_scalar(self.minimum)
    }

    fn commit_history(&mut self) {
        core::mem::swap(&mut self.previous_residual, &mut self.residual_workspace);
        core::mem::swap(
            &mut self.previous_relaxation,
            &mut self.relaxation_workspace,
        );
    }

    fn apply_slice(current: &mut [T], residual: &[T], relaxation: &[T]) {
        for ((current_value, residual), relaxation) in current
            .iter_mut()
            .zip(residual.iter().copied())
            .zip(relaxation.iter().copied())
        {
            *current_value = relaxation.scalar_fmadd(residual, *current_value);
        }
    }
}

/// Evaluate `-factor * residual / (denominator * 2^denominator_shift)`.
///
/// Separate the binary exponents before multiplying: each nonzero mantissa
/// has magnitude in [1, 2), the product is in [1, 4), and the quotient is in
/// (1/2, 4). Thus neither the product nor division loses its range before the
/// final scaling. FMA retains the product and division rounding residuals so
/// cancellation near MAX cannot turn a finite factor into a false infinity.
/// The final scaling keeps genuine factor overflow visible to the caller.
/// All arithmetic, including the rounding correction, stays in `T`.
fn scaled_secant<T: RealField>(
    factor: T,
    residual: T,
    denominator: T,
    denominator_shift: i32,
) -> Option<T> {
    if residual == <T as NumericElement>::ZERO {
        return Some(<T as NumericElement>::ZERO);
    }
    let factor_exponent = factor.binary_exponent()?;
    let residual_exponent = residual.binary_exponent()?;
    let denominator_exponent = denominator.binary_exponent()?;
    let factor = factor.scale_binary(-factor_exponent);
    let residual = residual.scale_binary(-residual_exponent);
    let denominator = denominator.scale_binary(-denominator_exponent);
    let product = factor * residual;
    let product_error = factor.scalar_fmadd(residual, -product);
    let quotient = product / denominator;
    let correction = ((-quotient).scalar_fmadd(denominator, product) + product_error) / denominator;
    let exponent = factor_exponent + residual_exponent - denominator_exponent - denominator_shift;
    Some(-(quotient + correction).scale_binary(exponent))
}

impl<T> Relaxation<T> for AitkenRelaxation<T>
where
    T: RealField,
{
    fn begin_window(&mut self) {
        self.previous_residual.clear();
        self.previous_relaxation.clear();
    }

    fn update_pair(
        &mut self,
        first_current: &mut [T],
        first_candidate: &[T],
        second_current: &mut [T],
        second_candidate: &[T],
    ) -> Result<(), RelaxationError> {
        validate_dimensions(first_current, first_candidate)?;
        validate_dimensions(second_current, second_candidate)?;

        let first_len = first_current.len();
        let total_len = first_len + second_current.len();
        let history_available = self.previous_residual.len() == total_len
            && self.previous_relaxation.len() == total_len;
        self.residual_workspace
            .resize(total_len, <T as NumericElement>::ZERO);
        self.relaxation_workspace
            .resize(total_len, <T as NumericElement>::ZERO);

        self.update_slice(first_current, first_candidate, 0, history_available)?;
        self.update_slice(
            second_current,
            second_candidate,
            first_len,
            history_available,
        )?;

        self.commit_history();
        Self::apply_slice(
            first_current,
            &self.previous_residual[..first_len],
            &self.previous_relaxation[..first_len],
        );
        Self::apply_slice(
            second_current,
            &self.previous_residual[first_len..],
            &self.previous_relaxation[first_len..],
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{AitkenRelaxation, Relaxation, RelaxationError, scaled_secant};
    use eunomia::{NumericElement, RealField};

    fn check_cancelled_factors<T: RealField>(maximum: T, smallest: T) {
        let one = <T as NumericElement>::ONE;
        let half = T::from_f64(0.5);
        for factor in [smallest, half, one, maximum] {
            for residual in [smallest, half, one, maximum] {
                // The exact rational oracle cancels equal operands. Neither
                // product overflow nor a subnormal product may change it.
                assert_eq!(scaled_secant(factor, residual, -residual, 0), Some(factor));
                assert_eq!(scaled_secant(factor, residual, residual, 0), Some(-factor));
            }
        }
        // Sweep non-power mantissas at MAX, where uncorrected multiplication
        // and division can round an exact cancellation up to infinity.
        for numerator in 128..256 {
            let residual = T::from_integer(numerator) / T::from_f64(128.0);
            assert_eq!(
                scaled_secant(maximum, residual, -residual, 0),
                Some(maximum)
            );
        }
    }

    #[test]
    fn mantissa_cancellation_preserves_large_and_subnormal_factors() {
        check_cancelled_factors(f32::MAX, f32::from_bits(1));
        check_cancelled_factors(f64::MAX, f64::from_bits(1));
    }

    fn check_nonfinite_factor<T: RealField>(maximum: T, smallest: T) {
        let zero = <T as NumericElement>::ZERO;
        let one = <T as NumericElement>::ONE;
        let half = T::from_f64(0.5);
        let mut policy =
            AitkenRelaxation::new(one, maximum, smallest).expect("finite positive bounds");
        policy.previous_residual = alloc::vec![one];
        policy.previous_relaxation = alloc::vec![maximum];
        let mut current = [zero];
        let mut other = [];
        // -MAX * 1 / (1/2 - 1) is genuinely out of range. Failure must
        // preserve both the caller's value and the last successful history.
        assert_eq!(
            policy.update_pair(&mut current, &[half], &mut other, &[]),
            Err(RelaxationError::NonFinite { index: 0 })
        );
        assert_eq!(current, [zero]);
        assert_eq!(policy.previous_residual, [one]);
        assert_eq!(policy.previous_relaxation, [maximum]);
        policy
            .update_pair(&mut current, &[zero], &mut other, &[])
            .expect("the same history gives a finite factor MAX for a zero residual");
        assert_eq!(current, [zero]);
        assert_eq!(policy.previous_residual, [zero]);
        assert_eq!(policy.previous_relaxation, [maximum]);
    }

    #[test]
    fn genuine_factor_overflow_keeps_the_transaction_and_error_contract() {
        check_nonfinite_factor(f32::MAX, f32::from_bits(1));
        check_nonfinite_factor(f64::MAX, f64::from_bits(1));
    }
}
