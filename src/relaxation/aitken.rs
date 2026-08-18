use alloc::vec::Vec;

use eunomia::{NumericElement, RealField};

use super::{InvalidAitkenRelaxation, Relaxation, RelaxationError};

/// Stateful componentwise Aitken \(\Delta^2\) relaxation for a coupled pair.
///
/// The policy stacks both interface residuals into one history vector. The
/// first update uses unit relaxation. Later updates use the Irons--Tuck
/// componentwise secant estimate
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
            let raw = -(previous_factor * previous_residual) / difference;
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
        self.previous_residual.clear();
        self.previous_residual
            .extend_from_slice(&self.residual_workspace);
        self.previous_relaxation.clear();
        self.previous_relaxation
            .extend_from_slice(&self.relaxation_workspace);
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

impl<T> Relaxation<T> for AitkenRelaxation<T>
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
        if first_current.len() != first_candidate.len() {
            return Err(RelaxationError::Dimension {
                current: first_current.len(),
                candidate: first_candidate.len(),
            });
        }
        if second_current.len() != second_candidate.len() {
            return Err(RelaxationError::Dimension {
                current: second_current.len(),
                candidate: second_candidate.len(),
            });
        }

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
            &self.residual_workspace[..first_len],
            &self.relaxation_workspace[..first_len],
        );
        Self::apply_slice(
            second_current,
            &self.residual_workspace[first_len..],
            &self.relaxation_workspace[first_len..],
        );
        Ok(())
    }
}
