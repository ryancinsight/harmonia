//! Generic-instantiation evidence across every Phase 0 scalar.

mod support;

use athena_core::{ConvergencePolicy, NoObserver};
use eunomia::RealField;
use harmonia::{FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace, PartitionedPair};

use support::{LinearPartition, instant, window};

fn solve_constant_pair<T>() -> [T; 2]
where
    T: RealField,
{
    let model = PairComponents::new(
        LinearPartition {
            source: T::from_f64(1.0),
            gain: T::from_f64(0.0),
        },
        LinearPartition {
            source: T::from_f64(-2.0),
            gain: T::from_f64(0.0),
        },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, T, 2, 3>::new(model, workspace).expect("valid subcycles");
    let mut first_state = [T::from_f64(0.25)];
    let mut second_state = [T::from_f64(0.75)];
    let mut first_input = [T::from_f64(0.0)];
    let mut second_input = [T::from_f64(0.0)];
    let policy = ConvergencePolicy::new(T::from_f64(0.0), T::from_f64(0.0), 2)
        .expect("invariant: valid policy");

    pair.solve_window(
        instant(),
        window(T::from_f64(0.5)),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut NoObserver,
    )
    .expect("constant pair converges");

    [first_state[0], second_state[0]]
}

fn assert_native_scalar_result<T>()
where
    T: RealField,
{
    let actual = solve_constant_pair::<T>();
    let tolerance = 8.0 * T::EPSILON.to_f64();
    assert!((actual[0].to_f64() - 0.75).abs() <= tolerance);
    assert!((actual[1].to_f64() + 0.25).abs() <= tolerance);
}

#[test]
fn all_phase_zero_scalar_monomorphizations_satisfy_the_contract() {
    assert_native_scalar_result::<f32>();
    assert_native_scalar_result::<f64>();
}
