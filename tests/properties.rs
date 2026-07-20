//! Generated contraction-law evidence.

mod support;

use athena_core::{ConvergencePolicy, NoObserver};
use harmonia::{FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace, PartitionedPair};
use proptest::prelude::*;

use support::{LinearPartition, euclidean_error, exact_interface, instant, window};

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
        let model = PairComponents::new(
            first,
            second,
            IdentityTransfer,
            IdentityTransfer,
            FullRelaxation,
        );
        let workspace = PairWorkspace::for_model(&model)
            .expect("generated model has compatible dimensions");
        let mut pair = PartitionedPair::<_, f64, 2, 3>::new(model, workspace)
            .expect("positive const subcycles");
        let mut first_state = [first_initial];
        let mut second_state = [second_initial];
        let mut first_input = [0.0];
        let mut second_input = [0.0];
        let policy = ConvergencePolicy::new(1.0e-11, 1.0e-11, 64)
            .expect("valid policy");

        let report = pair.solve_window(
            instant(),
            window(0.5),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        ).expect("generated map is contractive");

        let exact = exact_interface(first_initial, second_initial, 0.5, first, second);
        let contraction = (0.5 * first_gain).abs().max((0.5 * second_gain).abs());
        let rounding = 64.0 * f64::EPSILON
            * (1.0 + exact[0].abs() + exact[1].abs());
        let bound = report.residual_norm / (1.0 - contraction) + rounding;

        prop_assert!(euclidean_error([first_input[0], second_input[0]], exact) <= bound);
    }
}
