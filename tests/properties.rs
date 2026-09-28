//! Generated contraction-law evidence.

mod support;

use athena_core::ConvergencePolicy;
use proptest::prelude::*;

use support::{LinearPartition, Scaffold, euclidean_error, exact_interface, solve_linear_pair};

proptest! {
    #[test]
    fn contractive_linear_pairs_satisfy_a_posteriori_bound(
        first_source in -2.0_f64..2.0,
        second_source in -2.0_f64..2.0,
        first_gain in -0.4_f64..0.4,
        second_gain in -0.4_f64..0.4,
        first_initial in -1.0_f64..1.0,
        second_initial in -1.0_f64..1.0,
    ) {
        let first = LinearPartition { source: first_source, gain: first_gain };
        let second = LinearPartition { source: second_source, gain: second_gain };
        let mut scaffold = Scaffold::new(first_initial, second_initial);
        let policy = ConvergencePolicy::new(1.0e-11, 1.0e-11, 64)
            .expect("valid policy");

        let report = solve_linear_pair::<f64, 2, 3>(first, second, 0.5, &policy, &mut scaffold)
            .expect("generated map is contractive");

        let exact = exact_interface(first_initial, second_initial, 0.5, first, second);
        let contraction = (0.5 * first_gain).abs().max((0.5 * second_gain).abs());
        let rounding = 64.0 * f64::EPSILON
            * (1.0 + exact[0].abs() + exact[1].abs());
        let bound = report.residual_norm / (1.0 - contraction) + rounding;

        prop_assert!(
            euclidean_error([scaffold.first_input[0], scaffold.second_input[0]], exact) <= bound
        );
    }
}
