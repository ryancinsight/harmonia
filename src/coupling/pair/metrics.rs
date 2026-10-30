use eunomia::{NumericElement, RealField};

/// Native-precision Euclidean norms of the unrelaxed defect and candidate.
pub(super) fn pair_metrics<T: RealField>(
    first_current: &[T],
    first_candidate: &[T],
    second_current: &[T],
    second_candidate: &[T],
) -> Option<(T, T)> {
    let candidates = first_candidate.iter().chain(second_candidate).copied();
    let differences = first_current
        .iter()
        .chain(second_current)
        .copied()
        .zip(candidates.clone())
        .map(|(current, candidate)| candidate - current);
    Some((scaled_norm(differences)?, scaled_norm(candidates)?))
}

/// Normalize by the largest magnitude, retaining rounding residuals in `T`.
///
/// A maximum contributes exactly one to the scaled sum, so a nonzero norm
/// survives even when all unscaled squares would underflow. Ratios are at
/// most one; the sum is bounded by the addressable component count for the
/// supported f32/f64 fields. FMA recovers division/product residuals, and a
/// two-component sum retains low-order contributions through the reduction.
///
/// Correct the square root and its product before restoring a factor of two
/// for large scales. This keeps a rounded intermediate from overflowing when
/// the corrected norm is still finite. All arithmetic stays in `T`: the two
/// components store a rounding residual, not values in a wider scalar type.
fn scaled_norm<T: RealField>(values: impl Iterator<Item = T> + Clone) -> Option<T> {
    let zero = <T as NumericElement>::ZERO;
    let one = <T as NumericElement>::ONE;
    let two = one + one;
    let mut scale = zero;
    for value in values.clone() {
        if !value.is_finite() {
            return None;
        }
        scale = scale.max_scalar(value.abs());
    }
    if scale == zero {
        return Some(zero);
    }
    let mut sum = CompensatedSum {
        high: zero,
        low: zero,
    };
    for value in values {
        let magnitude = value.abs();
        let ratio = magnitude / scale;
        let ratio_error = (-ratio).scalar_fmadd(scale, magnitude) / scale;
        let square = ratio * ratio;
        let square_error = ratio.scalar_fmadd(ratio, -square)
            + ((ratio + ratio) * ratio_error + ratio_error * ratio_error);
        sum.add(square, square_error);
    }
    let root = sum.high.sqrt();
    let root_error = ((-root).scalar_fmadd(root, sum.high) + sum.low) / (two * root);
    // Below one, scaling by a half could erase a minimum subnormal. Above
    // one, it is exact and makes room for the rounding correction near MAX.
    let factor = if scale > one { one / two } else { one };
    let reduced_scale = scale * factor;
    let product = reduced_scale * root;
    if !product.is_finite() {
        return None;
    }
    let product_error = reduced_scale.scalar_fmadd(root, -product) + reduced_scale * root_error;
    let norm = (product + product_error) / factor;
    norm.is_finite().then_some(norm)
}

struct CompensatedSum<T> {
    high: T,
    low: T,
}

impl<T: RealField> CompensatedSum<T> {
    /// `TwoSum` recovers the addition residual; renormalize the carried tail.
    fn add(&mut self, high: T, low: T) {
        let sum = self.high + high;
        let virtual_high = sum - self.high;
        let error = (self.high - (sum - virtual_high)) + (high - virtual_high);
        let tail = (self.low + low) + error;
        self.high = sum + tail;
        self.low = tail - (self.high - sum);
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
