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

fn check_range_boundary<T: RealField>(first: T, second: T, upper: T, maximum: T) {
    let zero = <T as NumericElement>::ZERO;
    let one = <T as NumericElement>::ONE;
    let policy = ConvergencePolicy::new(zero, one, 1).expect("valid relative policy");
    for (source, finite) in [(second, true), (upper, false)] {
        let mut values = Scaffold::new(-zero, zero);
        let result = solve_linear_pair::<T, 1, 1>(
            LinearPartition {
                source: first,
                gain: T::from_f64(0.5),
            },
            LinearPartition {
                source,
                gain: T::from_f64(0.25),
            },
            one,
            &policy,
            &mut values,
        );
        if finite {
            let report = result.expect("the exact metric fits in the native scalar");
            assert_eq!(report.residual_norm, maximum);
            assert_eq!(report.threshold, maximum);
            assert_eq!(values.first_state[0], first);
            assert_eq!(values.second_state[0], source);
            assert_eq!(values.first_input[0], source);
            assert_eq!(values.second_input[0], first);
        } else {
            assert_eq!(result, Err(CouplingError::NonFiniteMetric));
            assert_eq!(values.first_state[0], zero);
            assert!(values.first_state[0].is_sign_negative());
            assert_eq!(values.second_state[0], zero);
            assert!(values.second_state[0].is_sign_positive());
            assert_eq!(values.first_input[0], zero);
            assert!(values.first_input[0].is_sign_positive());
            assert_eq!(values.second_input[0], zero);
            assert!(values.second_input[0].is_sign_positive());
        }
    }
}

#[test]
fn near_maximum_metrics_preserve_both_sides_of_the_range_boundary() {
    // In each pair, x has exponent MAX_EXP-2 and y has exponent MAX_EXP.
    // At the common quantum, their integer significands are x_sig, 4*y_sig;
    // MAX is 4*max_sig. The output ULP is four quanta, hence midpoints +/-2.
    // All squared integers fit in u128 (at most 110 bits): this is an exact
    // certificate independent of the floating-point implementation under test.
    for (fraction_bits, x_bits, y_bits, gap) in [
        (
            52,
            0x7fca_e147_ae14_7adc_u64,
            0x7fef_4954_6ab6_de35_u64,
            53_289_514_649_133_936_u128,
        ),
        (23, 0x7e57_0a1a_u64, 0x7f7a_4aa4_u64, 112_878_700_u128),
    ] {
        let mask = (1_u128 << fraction_bits) - 1;
        let x = (u128::from(x_bits) & mask) | (1 << fraction_bits);
        let y = ((u128::from(y_bits) & mask) | (1 << fraction_bits)) * 4;
        let maximum = ((1_u128 << (fraction_bits + 1)) - 1) * 4;
        let square = x * x + y * y;
        assert_eq!(maximum * maximum - square, gap);
        assert!(square > (maximum - 2) * (maximum - 2));
        assert!(x * x + (y + 4) * (y + 4) > (maximum + 2) * (maximum + 2));
    }
    check_range_boundary(
        f64::from_bits(0x7fca_e147_ae14_7adc),
        f64::from_bits(0x7fef_4954_6ab6_de35),
        f64::from_bits(0x7fef_4954_6ab6_de36),
        f64::MAX,
    );
    check_range_boundary(
        f32::from_bits(0x7e57_0a1a),
        f32::from_bits(0x7f7a_4aa4),
        f32::from_bits(0x7f7a_4aa5),
        f32::MAX,
    );
}
