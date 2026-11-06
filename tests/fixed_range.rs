//! Value-semantic fixed-relaxation range witnesses.

use core::fmt::Debug;

use eunomia::{F32, F64, NumericElement, RealField};
use harmonia::{FixedRelaxation, FullRelaxation, Relaxation, RelaxationError};

fn unit_weight_matches_full<T: RealField + Debug + PartialEq>(maximum: T) {
    let one = <T as NumericElement>::ONE;
    let mut fixed_first = [maximum];
    let mut fixed_second = [-maximum];
    let mut full_first = fixed_first;
    let mut full_second = fixed_second;
    FullRelaxation
        .update_pair(&mut full_first, &[one], &mut full_second, &[-one])
        .expect("finite candidates are accepted by full relaxation");
    let mut fixed = FixedRelaxation::new(one).expect("unit weight is valid");
    fixed
        .update_pair(&mut fixed_first, &[one], &mut fixed_second, &[-one])
        .expect("unit weight accepts finite candidates");
    assert_eq!(fixed_first, full_first);
    assert_eq!(fixed_second, full_second);
}

fn opposite_extreme_midpoint_is_finite<T: RealField + Debug + PartialEq>(maximum: T) {
    let mut first = [-maximum];
    let mut second = [maximum];
    let mut fixed = FixedRelaxation::new(T::from_f64(0.5)).expect("half weight is valid");
    fixed
        .update_pair(&mut first, &[maximum], &mut second, &[-maximum])
        .expect("the exact midpoint of finite opposite extrema is zero");
    assert_eq!(first, [<T as NumericElement>::ZERO]);
    assert_eq!(second, [<T as NumericElement>::ZERO]);
}

#[test]
fn unit_weight_returns_candidates_at_single_precision() {
    unit_weight_matches_full(f32::MAX);
}

#[test]
fn unit_weight_returns_candidates_at_double_precision() {
    unit_weight_matches_full(f64::MAX);
}

#[test]
fn midpoint_of_opposite_extrema_is_finite_at_single_precision() {
    opposite_extreme_midpoint_is_finite(f32::MAX);
}

#[test]
fn midpoint_of_opposite_extrema_is_finite_at_double_precision() {
    opposite_extreme_midpoint_is_finite(f64::MAX);
}

#[test]
fn wrapped_scalars_preserve_the_same_range_contract() {
    unit_weight_matches_full(F32(f32::MAX));
    unit_weight_matches_full(F64(f64::MAX));
    opposite_extreme_midpoint_is_finite(F32(f32::MAX));
    opposite_extreme_midpoint_is_finite(F64(f64::MAX));
}

#[test]
fn cancellation_matches_the_exact_single_precision_oracle() {
    // Exact rational evaluation followed by one binary32 rounding.
    let current = f32::from_bits(0x7f16_1c84);
    let candidate = f32::from_bits(0xfeea_8ecd);
    let weight = f32::from_bits(0x3f0f_b789);
    let mut first = [current];
    let mut second = [];
    FixedRelaxation::new(weight)
        .expect("the fraction is within the unit interval")
        .update_pair(&mut first, &[candidate], &mut second, &[])
        .expect("the convex result is finite");
    assert_eq!(first[0].to_bits(), 0x72fa_cc03);
}

#[test]
fn cancellation_matches_the_exact_double_precision_oracle() {
    // Exact rational evaluation followed by one binary64 rounding.
    let current = f64::from_bits(0x7fd2_08cb_5078_76a8);
    let candidate = f64::from_bits(0xffe8_b102_3c7d_8967);
    let weight = f64::from_bits(0x3fd1_1ec7_67e5_1f55);
    let mut first = [current];
    let mut second = [];
    FixedRelaxation::new(weight)
        .expect("the fraction is within the unit interval")
        .update_pair(&mut first, &[candidate], &mut second, &[])
        .expect("the convex result is finite");
    assert_eq!(first[0].to_bits(), 0x7b8d_5738_08e9_0000);
}

#[test]
fn cancellation_preserves_the_low_product_terms() {
    // The exact rational result rounds to 0xf16c738d; dropping low product
    // terms rounds this witness to the adjacent value 0xf16c738c instead.
    let current = f32::from_bits(0xfd30_d355);
    let candidate = f32::from_bits(0x7f79_9637);
    let weight = f32::from_bits(0x3d2d_adbe);
    let mut first = [current];
    let mut second = [];
    FixedRelaxation::new(weight)
        .expect("the fraction is within the unit interval")
        .update_pair(&mut first, &[candidate], &mut second, &[])
        .expect("the convex result is finite");
    assert_eq!(first[0].to_bits(), 0xf16c_738d);
}

#[test]
fn unit_weight_preserves_candidate_signed_zero() {
    let mut first = [f64::MAX];
    let mut second = [-f64::MAX];
    FixedRelaxation::new(1.0)
        .expect("unit weight is valid")
        .update_pair(&mut first, &[-0.0], &mut second, &[0.0])
        .expect("finite candidates are accepted");
    assert_eq!(first[0].to_bits(), (-0.0_f64).to_bits());
    assert_eq!(second[0].to_bits(), 0.0_f64.to_bits());
}

#[test]
fn unit_weight_keeps_both_interfaces_on_nonfinite_input() {
    let mut first = [2.0_f64];
    let mut second = [f64::INFINITY];
    let result = FixedRelaxation::new(1.0)
        .expect("unit weight is valid")
        .update_pair(&mut first, &[4.0], &mut second, &[1.0]);
    assert_eq!(result, Err(RelaxationError::NonFinite { index: 1 }));
    assert_eq!(first[0].to_bits(), 2.0_f64.to_bits());
    assert_eq!(second[0].to_bits(), f64::INFINITY.to_bits());
}

#[test]
fn representable_differences_keep_the_fused_single_precision_result() {
    for (current, candidate, weight) in [
        (1.25_f32, 3.75_f32, 0.3_f32),
        (-f32::MAX, 0.0, 0.75),
        (f32::MIN_POSITIVE, -f32::MIN_POSITIVE, 0.25),
    ] {
        let expected = weight.mul_add(candidate - current, current);
        let mut first = [current];
        FixedRelaxation::new(weight)
            .expect("the fraction is within the unit interval")
            .update_pair(&mut first, &[candidate], &mut [], &[])
            .expect("the update is finite");
        assert_eq!(first[0].to_bits(), expected.to_bits());
    }
}

#[test]
fn representable_differences_keep_the_fused_double_precision_result() {
    for (current, candidate, weight) in [
        (1.25_f64, 3.75_f64, 0.3_f64),
        (-f64::MAX, 0.0, 0.75),
        (f64::MIN_POSITIVE, -f64::MIN_POSITIVE, 0.25),
    ] {
        let expected = weight.mul_add(candidate - current, current);
        let mut first = [current];
        FixedRelaxation::new(weight)
            .expect("the fraction is within the unit interval")
            .update_pair(&mut first, &[candidate], &mut [], &[])
            .expect("the update is finite");
        assert_eq!(first[0].to_bits(), expected.to_bits());
    }
}

#[test]
fn finite_difference_cancellation_retains_single_precision_low_term() {
    // (-1) * 2^-24 + (1 - 2^-24) * 2^-24 = -2^-48 exactly.
    let candidate = f32::from_bits((127 - 24) << 23);
    let weight = f32::from_bits(0x3f7f_ffff);
    let expected = -f32::from_bits((127 - 48) << 23);
    let mut first = [-1.0_f32];
    FixedRelaxation::new(weight)
        .expect("the weight is in the unit interval")
        .update_pair(&mut first, &[candidate], &mut [], &[])
        .expect("the exact convex result is finite");
    assert_eq!(first[0].to_bits(), expected.to_bits());
}

#[test]
fn finite_difference_cancellation_retains_double_precision_low_term() {
    // (-1) * 2^-53 + (1 - 2^-53) * 2^-53 = -2^-106 exactly.
    let candidate = f64::from_bits((1023 - 53) << 52);
    let weight = f64::from_bits(0x3fef_ffff_ffff_ffff);
    let expected = -f64::from_bits((1023 - 106) << 52);
    let mut first = [-1.0_f64];
    FixedRelaxation::new(weight)
        .expect("the weight is in the unit interval")
        .update_pair(&mut first, &[candidate], &mut [], &[])
        .expect("the exact convex result is finite");
    assert_eq!(first[0].to_bits(), expected.to_bits());
}

#[test]
fn finite_difference_correction_rounds_subnormal_ties() {
    let mut single = [f32::from_bits(1)];
    FixedRelaxation::new(0.5_f32)
        .expect("half weight is valid")
        .update_pair(&mut single, &[2.0 * f32::MIN_POSITIVE], &mut [], &[])
        .expect("the exact convex result is finite");
    assert_eq!(single[0].to_bits(), f32::MIN_POSITIVE.to_bits());

    let mut double = [f64::from_bits(1)];
    FixedRelaxation::new(0.5_f64)
        .expect("half weight is valid")
        .update_pair(&mut double, &[2.0 * f64::MIN_POSITIVE], &mut [], &[])
        .expect("the exact convex result is finite");
    assert_eq!(double[0].to_bits(), f64::MIN_POSITIVE.to_bits());
}

#[test]
fn finite_difference_correction_keeps_a_small_positive_candidate() {
    let mut single = [1.0_f32];
    let single_candidate = f32::from_bits((127 - 25) << 23);
    FixedRelaxation::new(f32::from_bits(0x3f7f_ffff))
        .expect("the weight is in the unit interval")
        .update_pair(&mut single, &[single_candidate], &mut [], &[])
        .expect("the exact convex result is finite");
    assert_eq!(single[0].to_bits(), (3.0 * single_candidate).to_bits());

    let mut double = [1.0_f64];
    let double_candidate = f64::from_bits((1023 - 54) << 52);
    FixedRelaxation::new(f64::from_bits(0x3fef_ffff_ffff_ffff))
        .expect("the weight is in the unit interval")
        .update_pair(&mut double, &[double_candidate], &mut [], &[])
        .expect("the exact convex result is finite");
    assert_eq!(double[0].to_bits(), (3.0 * double_candidate).to_bits());
}
