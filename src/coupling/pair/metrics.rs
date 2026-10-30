use eunomia::{NumericElement, RealField};

/// Native-precision Euclidean norms of the unrelaxed defect and candidate.
pub(super) fn pair_metrics<T: RealField>(
    first_current: &[T],
    first_candidate: &[T],
    second_current: &[T],
    second_candidate: &[T],
) -> Option<(T, T)> {
    let mut residual = ScaledNorm::new();
    let mut candidate_norm = ScaledNorm::new();
    for (current, candidate) in first_current
        .iter()
        .chain(second_current)
        .copied()
        .zip(first_candidate.iter().chain(second_candidate).copied())
    {
        residual.push(candidate - current)?;
        candidate_norm.push(candidate)?;
    }
    Some((residual.finish()?, candidate_norm.finish()?))
}

/// Maintain `sum(x_i²) = scale² * sum` without squaring an unscaled input.
///
/// When a new magnitude `a` exceeds `scale`, division by `a` gives the new
/// invariant `sum' = 1 + sum * (scale/a)²`; otherwise add `(a/scale)²`.
/// Every ratio is at most one and a nonzero maximum contributes exactly one,
/// so squaring cannot erase the entire norm or overflow at the input scale.
/// `sum` is bounded by the number of components (representable for the native
/// f32/f64 fields and any addressable slice). Rounding remains in `T`.
/// Nonfinite components or an unrepresentable final norm remain errors.
struct ScaledNorm<T> {
    scale: T,
    sum: T,
}

impl<T: RealField> ScaledNorm<T> {
    fn new() -> Self {
        Self {
            scale: <T as NumericElement>::ZERO,
            sum: <T as NumericElement>::ONE,
        }
    }

    fn push(&mut self, value: T) -> Option<()> {
        if !value.is_finite() {
            return None;
        }
        let magnitude = value.abs();
        if magnitude > self.scale {
            let ratio = self.scale / magnitude;
            self.sum = (self.sum * ratio).scalar_fmadd(ratio, <T as NumericElement>::ONE);
            self.scale = magnitude;
        } else if magnitude > <T as NumericElement>::ZERO {
            let ratio = magnitude / self.scale;
            self.sum = ratio.scalar_fmadd(ratio, self.sum);
        }
        Some(())
    }

    fn finish(self) -> Option<T> {
        let norm = self.scale * self.sum.sqrt();
        norm.is_finite().then_some(norm)
    }
}

#[cfg(test)]
mod tests {
    use super::pair_metrics;

    #[test]
    fn combined_interfaces_match_a_scaled_pythagorean_oracle() {
        for unit in [2.0_f64.powi(-600), 1.0, 2.0_f64.powi(600)] {
            for coefficients in [[3.0, -4.0, 12.0], [12.0, -4.0, 3.0]] {
                let candidate = coefficients.map(|value| value * unit);
                let (residual, norm) =
                    pair_metrics(&[0.0; 2], &candidate[..2], &[0.0], &candidate[2..])
                        .expect("the 3-4-12-13 norm is finite at this scale");
                // Four rounded operations along each scaled contribution and
                // the final square root/product permit eight relative epsilons.
                assert!((norm / unit - 13.0).abs() <= 8.0 * f64::EPSILON * 13.0);
                assert_eq!(residual.to_bits(), norm.to_bits());
                assert_eq!(
                    pair_metrics(
                        &candidate[..2],
                        &candidate[..2],
                        &candidate[2..],
                        &candidate[2..]
                    ),
                    Some((0.0, norm))
                );
            }
        }
        assert_eq!(
            pair_metrics(&[-0.0], &[0.0], &[0.0], &[-0.0]),
            Some((0.0, 0.0))
        );
    }

    #[test]
    fn subtraction_overflow_and_nonfinite_inputs_are_rejected() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(pair_metrics(&[invalid], &[1.0], &[0.0], &[0.0]), None);
            assert_eq!(pair_metrics(&[0.0], &[0.0], &[1.0], &[invalid]), None);
        }
        assert_eq!(
            pair_metrics(&[-f64::MAX], &[f64::MAX], &[0.0], &[0.0]),
            None
        );
    }
}
