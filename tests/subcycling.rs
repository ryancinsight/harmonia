//! Differential evidence across const-generic subcycle specializations.

mod support;

use athena_core::ConvergencePolicy;

use support::{LinearPartition, Scaffold, solve_linear_pair};

fn solve<const FIRST: usize, const SECOND: usize>() -> ([f64; 2], [f64; 2]) {
    let mut scaffold = Scaffold::new(0.25, 0.75);
    let policy = ConvergencePolicy::new(0.0, 0.0, 2).expect("invariant: valid iteration policy");

    solve_linear_pair::<f64, FIRST, SECOND>(
        LinearPartition {
            source: 1.0_f64,
            gain: 0.0,
        },
        LinearPartition {
            source: -2.0_f64,
            gain: 0.0,
        },
        0.5,
        &policy,
        &mut scaffold,
    )
    .expect("constant derivative pair converges");

    (
        [scaffold.first_state[0], scaffold.second_state[0]],
        [scaffold.first_input[0], scaffold.second_input[0]],
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
