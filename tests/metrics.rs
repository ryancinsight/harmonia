//! Euclidean-defect acceptance across the native scalar exponent range.

mod support;

use athena_core::ConvergencePolicy;
use eunomia::{NumericElement, RealField};
use harmonia::CouplingError;
use support::{LinearPartition, Scaffold, solve_linear_pair};

fn check_scaled_defect<T>(unit: T)
where
    T: RealField,
{
    let zero = <T as NumericElement>::ZERO;
    let one = <T as NumericElement>::ONE;
    let first = LinearPartition {
        source: T::from_f64(3.0) * unit,
        gain: T::from_f64(0.5),
    };
    let second = LinearPartition {
        source: T::from_f64(4.0) * unit,
        gain: T::from_f64(0.25),
    };
    // One unit of time gives F(0) = (4u, 3u), hence ||F(0)|| = 5u.
    // The nonzero gains make these real coupled linear evolution equations.
    let expected = T::from_f64(5.0) * unit;
    let mut values = Scaffold::new(zero, zero);
    let policy = ConvergencePolicy::new(zero, zero, 1).expect("valid exact policy");
    let result = solve_linear_pair::<T, 1, 1>(first, second, one, &policy, &mut values);
    let Err(CouplingError::NotConverged {
        iterations,
        residual_norm,
        threshold,
    }) = result
    else {
        panic!("a representable nonzero defect must exhaust the exact-policy budget: {result:?}");
    };
    assert_eq!(iterations, 1);
    assert_eq!(residual_norm, expected);
    assert_eq!(threshold, zero);
    assert_eq!(values.first_state, [zero]);
    assert_eq!(values.second_state, [zero]);
    assert_eq!(values.first_input, [zero]);
    assert_eq!(values.second_input, [zero]);

    // A relative tolerance of one accepts this same defect, and exposes the
    // candidate norm through the threshold rather than just its finiteness.
    let policy = ConvergencePolicy::new(zero, one, 1).expect("valid relative policy");
    let report = solve_linear_pair::<T, 1, 1>(first, second, one, &policy, &mut values)
        .expect("the finite candidate norm equals the defect norm");
    assert_eq!(report.iterations, 1);
    assert_eq!(report.residual_norm, expected);
    assert_eq!(report.threshold, expected);
    assert_eq!(values.first_state, [first.source]);
    assert_eq!(values.second_state, [second.source]);
    assert_eq!(values.first_input, [second.source]);
    assert_eq!(values.second_input, [first.source]);
}

#[test]
fn tiny_defects_do_not_manufacture_convergence() {
    check_scaled_defect(2.0_f32.powi(-80));
    check_scaled_defect(2.0_f64.powi(-600));
}

#[test]
fn finite_large_norms_do_not_overflow_their_squares() {
    check_scaled_defect(2.0_f32.powi(80));
    check_scaled_defect(2.0_f64.powi(600));
}

#[test]
fn subnormal_defects_remain_observable() {
    check_scaled_defect(f32::from_bits(1));
    check_scaled_defect(f64::from_bits(1));
}

#[test]
fn every_binary_scale_with_a_representable_pythagorean_norm() {
    // 5 * 2^e stays finite through e = 125 / 1021. These bit patterns
    // enumerate subnormal powers directly and normal powers by exponent.
    for bit in 0..23 {
        check_scaled_defect(f32::from_bits(1 << bit));
    }
    for exponent in 1..=252 {
        check_scaled_defect(f32::from_bits(exponent << 23));
    }
    for bit in 0..52 {
        check_scaled_defect(f64::from_bits(1 << bit));
    }
    for exponent in 1..=2044 {
        check_scaled_defect(f64::from_bits(exponent << 52));
    }
}

#[test]
fn nonfinite_metrics_leave_all_caller_bits_unchanged() {
    for (first_source, second_source) in [
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
        (f64::NEG_INFINITY, 1.0),
        // Finite components whose *norm* really exceeds the scalar range.
        (f64::MAX, f64::MAX),
    ] {
        let mut values = Scaffold::new(-0.0_f64, 0.0);
        let before = [
            values.first_state[0],
            values.second_state[0],
            values.first_input[0],
            values.second_input[0],
        ]
        .map(f64::to_bits);
        let policy = ConvergencePolicy::new(0.0, 0.0, 1).expect("valid policy");
        let result = solve_linear_pair::<f64, 1, 1>(
            LinearPartition {
                source: first_source,
                gain: 0.5,
            },
            LinearPartition {
                source: second_source,
                gain: 0.25,
            },
            1.0,
            &policy,
            &mut values,
        );
        assert_eq!(result, Err(CouplingError::NonFiniteMetric));
        assert_eq!(
            [
                values.first_state[0],
                values.second_state[0],
                values.first_input[0],
                values.second_input[0],
            ]
            .map(f64::to_bits),
            before
        );
    }
}
