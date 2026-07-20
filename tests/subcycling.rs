//! Differential evidence across const-generic subcycle specializations.

mod support;

use athena_core::{ConvergencePolicy, NoObserver};
use harmonia::{FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace, PartitionedPair};

use support::{LinearPartition, instant, window};

fn solve<const FIRST: usize, const SECOND: usize>() -> ([f64; 2], [f64; 2]) {
    let model = PairComponents::new(
        LinearPartition {
            source: 1.0_f64,
            gain: 0.0,
        },
        LinearPartition {
            source: -2.0_f64,
            gain: 0.0,
        },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair =
        PartitionedPair::<_, f64, FIRST, SECOND>::new(model, workspace).expect("valid subcycles");
    let mut first_state = [0.25];
    let mut second_state = [0.75];
    let mut first_input = [0.0];
    let mut second_input = [0.0];
    let policy = ConvergencePolicy::new(0.0, 0.0, 2).expect("invariant: valid iteration policy");

    pair.solve_window(
        instant(),
        window(0.5),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut NoObserver,
    )
    .expect("constant derivative pair converges");

    (
        [first_state[0], second_state[0]],
        [first_input[0], second_input[0]],
    )
}

#[test]
fn heterogeneous_subcycle_monomorphizations_share_endpoint_semantics() {
    let coarse = solve::<1, 1>();
    let heterogeneous = solve::<2, 3>();
    let tolerance = 8.0 * f64::EPSILON;

    for (left, right) in coarse
        .0
        .into_iter()
        .chain(coarse.1)
        .zip(heterogeneous.0.into_iter().chain(heterogeneous.1))
    {
        assert!((left - right).abs() <= tolerance);
    }
    assert_eq!(coarse.0[0].to_bits(), 0.75_f64.to_bits());
    assert_eq!(coarse.0[1].to_bits(), (-0.25_f64).to_bits());
    assert_eq!(coarse.1[0].to_bits(), (-0.25_f64).to_bits());
    assert_eq!(coarse.1[1].to_bits(), 0.75_f64.to_bits());
}
