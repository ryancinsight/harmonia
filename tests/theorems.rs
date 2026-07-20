//! Executable evidence for Phase 0 coupling theorems.

mod support;

use athena_core::ConvergencePolicy;
use harmonia::{
    CouplingError, FixedRelaxation, FullRelaxation, IdentityTransfer, PairComponents,
    PairWorkspace, PartitionedPair,
};

use support::{
    ConstantOutput, LastObserver, LinearPartition, euclidean_error, exact_interface, instant,
    window,
};

#[test]
fn contraction_residual_bounds_fixed_point_error() {
    let first = LinearPartition {
        source: 1.25_f64,
        gain: 0.2,
    };
    let second = LinearPartition {
        source: -0.5_f64,
        gain: -0.3,
    };
    let model = PairComponents::new(
        first,
        second,
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 2, 3>::new(model, workspace)
        .expect("invariant: positive subcycle ratios");
    let mut first_state = [0.4];
    let mut second_state = [-0.1];
    let initial_first = first_state[0];
    let initial_second = second_state[0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];
    let policy = ConvergencePolicy::new(1.0e-12, 1.0e-12, 64).expect("invariant: valid policy");
    let mut observer = LastObserver::default();

    let report = pair
        .solve_window(
            instant(),
            window(0.5),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut observer,
        )
        .expect("contractive pair converges");

    let exact = exact_interface(initial_first, initial_second, 0.5, first, second);
    let actual = [first_input[0], second_input[0]];
    let contraction = (0.5 * first.gain).abs().max((0.5 * second.gain).abs());
    let rounding = 32.0 * f64::EPSILON * (1.0 + exact[0].abs() + exact[1].abs());
    let theorem_bound = report.residual_norm / (1.0 - contraction) + rounding;

    assert!(euclidean_error(actual, exact) <= theorem_bound);
    assert!((first_state[0] - exact[1]).abs() <= theorem_bound);
    assert!((second_state[0] - exact[0]).abs() <= theorem_bound);
    assert_eq!(observer.count, report.iterations);
}

#[test]
fn nonconvergence_is_transactional() {
    let model = PairComponents::new(
        ConstantOutput { output: 2.0_f64 },
        ConstantOutput { output: -3.0_f64 },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("valid subcycles");
    let mut first_state = [11.0];
    let mut second_state = [12.0];
    let mut first_input = [13.0];
    let mut second_input = [14.0];
    let before = (first_state, second_state, first_input, second_input);
    let policy = ConvergencePolicy::new(0.0, 0.0, 1).expect("invariant: one iteration is valid");

    let result = pair.solve_window(
        instant(),
        window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut LastObserver::default(),
    );

    assert!(matches!(
        result,
        Err(CouplingError::NotConverged { iterations: 1, .. })
    ));
    assert_eq!(
        (first_state, second_state, first_input, second_input),
        before
    );
}

#[test]
fn relaxation_weight_cannot_manufacture_convergence() {
    let relaxation =
        FixedRelaxation::new(1.0e-12_f64).expect("invariant: positive unit-interval weight");
    let model = PairComponents::new(
        ConstantOutput { output: 1.0_f64 },
        ConstantOutput { output: 1.0_f64 },
        IdentityTransfer,
        IdentityTransfer,
        relaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("valid subcycles");
    let policy = ConvergencePolicy::new(1.0e-6, 0.0, 2).expect("invariant: valid tolerance");
    let mut first_state = [0.0];
    let mut second_state = [0.0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];

    let result = pair.solve_window(
        instant(),
        window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut LastObserver::default(),
    );

    let Err(CouplingError::NotConverged { residual_norm, .. }) = result else {
        panic!("raw defect must remain above tolerance");
    };
    assert!(residual_norm > 1.0);
}
