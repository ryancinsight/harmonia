//! Analytical and transactional coverage for provider-owned Aitken relaxation.

use eunomia::RealField;
use harmonia::{AitkenRelaxation, InvalidAitkenRelaxation, Relaxation, RelaxationError};

fn assert_close<T>(actual: T, expected: f64)
where
    T: RealField,
{
    let tolerance = 16.0 * T::EPSILON.to_f64();
    assert!((actual.to_f64() - expected).abs() <= tolerance);
}

#[test]
fn aitken_matches_the_componentwise_secant_oracle_across_the_pair() {
    let mut policy = AitkenRelaxation::new(0.05_f64, 1.5, 1.0e-12)
        .expect("invariant: valid Aitken configuration");
    let mut first_current = [0.0];
    let mut second_current = [0.0];

    policy
        .update_pair(&mut first_current, &[1.0], &mut second_current, &[2.0])
        .expect("first Aitken update is finite");
    assert_eq!(first_current[0].to_bits(), 1.0_f64.to_bits());
    assert_eq!(second_current[0].to_bits(), 2.0_f64.to_bits());

    policy
        .update_pair(&mut first_current, &[0.5], &mut second_current, &[1.0])
        .expect("second Aitken update is finite");
    assert_close(first_current[0], 2.0 / 3.0);
    assert_close(second_current[0], 4.0 / 3.0);
}

#[test]
fn aitken_clamps_secant_factors_to_both_configured_bounds() {
    let mut initial_policy = AitkenRelaxation::new(0.5_f64, 0.75, 1.0e-12)
        .expect("invariant: valid initial-factor bounds");
    let mut initial_current = [0.0];
    let mut initial_other = [0.0];
    initial_policy
        .update_pair(&mut initial_current, &[1.0], &mut initial_other, &[1.0])
        .expect("initial clamped update is finite");
    assert_eq!(initial_current[0].to_bits(), 0.75_f64.to_bits());

    let mut policy = AitkenRelaxation::new(0.05_f64, 1.5, 1.0e-12)
        .expect("invariant: valid Aitken configuration");
    let mut current = [0.0];
    let mut other = [0.0];

    policy
        .update_pair(&mut current, &[1.0], &mut other, &[1.0])
        .expect("first Aitken update is finite");
    policy
        .update_pair(&mut current, &[1.9], &mut other, &[1.1])
        .expect("upper-clamped Aitken update is finite");
    assert_close(current[0], 1.0 + 1.5 * 0.9);

    let mut lower_policy = AitkenRelaxation::new(0.05_f64, 1.5, 1.0e-12)
        .expect("invariant: valid lower-bound configuration");
    let mut current = [0.0];
    let mut other = [0.0];
    lower_policy
        .update_pair(&mut current, &[1.0], &mut other, &[1.0])
        .expect("history restart is finite");
    lower_policy
        .update_pair(&mut current, &[2.1], &mut other, &[1.1])
        .expect("lower-clamped Aitken update is finite");
    assert_close(current[0], 1.0 + 0.05 * 1.1);
}

#[test]
fn aitken_reuses_the_previous_factor_for_a_small_residual_difference() {
    let mut policy = AitkenRelaxation::new(0.05_f64, 1.5, 1.0e-12)
        .expect("invariant: valid Aitken configuration");
    let mut first = [0.0];
    let mut second = [0.0];

    policy
        .update_pair(&mut first, &[1.0], &mut second, &[1.0])
        .expect("first Aitken update is finite");
    policy
        .update_pair(&mut first, &[1.0 + 1.0e-13], &mut second, &[1.0 + 1.0e-13])
        .expect("small-denominator update is finite");
    assert_eq!(first[0].to_bits(), (1.0 + 1.0e-13_f64).to_bits());
    assert_eq!(second[0].to_bits(), (1.0 + 1.0e-13_f64).to_bits());
}

#[test]
fn aitken_rejects_invalid_configuration() {
    assert!(matches!(
        AitkenRelaxation::new(f64::NAN, 1.0, 1.0e-12),
        Err(InvalidAitkenRelaxation::NonFiniteBound)
    ));
    assert!(matches!(
        AitkenRelaxation::new(0.0, 1.0, 1.0e-12),
        Err(InvalidAitkenRelaxation::NonPositiveMinimum)
    ));
    assert!(matches!(
        AitkenRelaxation::new(1.0, 0.5, 1.0e-12),
        Err(InvalidAitkenRelaxation::MaximumBelowMinimum)
    ));
    assert!(matches!(
        AitkenRelaxation::new(0.05, 1.5, 0.0),
        Err(InvalidAitkenRelaxation::NonPositiveTolerance)
    ));
}

#[test]
fn aitken_failures_are_transactional_for_interfaces_and_history() {
    let mut policy = AitkenRelaxation::new(0.05_f64, 1.5, 1.0e-12)
        .expect("invariant: valid Aitken configuration");
    let mut first = [0.0];
    let mut second = [0.0];
    policy
        .update_pair(&mut first, &[1.0], &mut second, &[2.0])
        .expect("first Aitken update is finite");

    let first_before = first;
    let second_before = second;
    assert_eq!(
        policy.update_pair(&mut first, &[0.5], &mut second, &[f64::NAN]),
        Err(RelaxationError::NonFinite { index: 1 })
    );
    assert_eq!(first[0].to_bits(), first_before[0].to_bits());
    assert_eq!(second[0].to_bits(), second_before[0].to_bits());

    policy
        .update_pair(&mut first, &[0.5], &mut second, &[1.0])
        .expect("history remains usable after rejected update");
    assert_close(first[0], 2.0 / 3.0);
    assert_close(second[0], 4.0 / 3.0);
}

#[test]
fn aitken_reports_pair_dimensions_without_mutating_state() {
    let mut policy = AitkenRelaxation::new(0.05_f64, 1.5, 1.0e-12)
        .expect("invariant: valid Aitken configuration");
    let mut first = [2.0];
    let mut second = [3.0];
    assert_eq!(
        policy.update_pair(&mut first, &[], &mut second, &[3.0]),
        Err(RelaxationError::Dimension {
            current: 1,
            candidate: 0,
        })
    );
    assert_eq!(first[0].to_bits(), 2.0_f64.to_bits());
    assert_eq!(second[0].to_bits(), 3.0_f64.to_bits());
}

fn generic_pair_update<T>()
where
    T: RealField,
{
    let mut policy =
        AitkenRelaxation::new(T::from_f64(0.05), T::from_f64(1.5), T::from_f64(1.0e-12))
            .expect("invariant: valid native-precision configuration");
    let mut first = [T::from_f64(0.0)];
    let mut second = [T::from_f64(0.0)];
    policy
        .update_pair(
            &mut first,
            &[T::from_f64(1.0)],
            &mut second,
            &[T::from_f64(2.0)],
        )
        .expect("first native-precision update is finite");
    policy
        .update_pair(
            &mut first,
            &[T::from_f64(0.5)],
            &mut second,
            &[T::from_f64(1.0)],
        )
        .expect("second native-precision update is finite");
    assert_close(first[0], 2.0 / 3.0);
    assert_close(second[0], 4.0 / 3.0);
}

#[test]
fn aitken_is_instantiated_for_each_shipped_real_scalar() {
    generic_pair_update::<f32>();
    generic_pair_update::<f64>();
}
