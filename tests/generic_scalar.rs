//! Generic-instantiation evidence across every Phase 0 scalar.

mod support;

use athena_core::ConvergencePolicy;
use eunomia::RealField;

use support::{LinearPartition, Scaffold, solve_linear_pair};

fn solve_constant_pair<T>() -> [T; 2]
where
    T: RealField,
{
    let mut scaffold = Scaffold::new(T::from_f64(0.25), T::from_f64(0.75));
    let policy = ConvergencePolicy::new(T::from_f64(0.0), T::from_f64(0.0), 2)
        .expect("invariant: valid policy");

    solve_linear_pair::<T, 2, 3>(
        LinearPartition {
            source: T::from_f64(1.0),
            gain: T::from_f64(0.0),
        },
        LinearPartition {
            source: T::from_f64(-2.0),
            gain: T::from_f64(0.0),
        },
        T::from_f64(0.5),
        &policy,
        &mut scaffold,
    )
    .expect("constant pair converges");

    [scaffold.first_state[0], scaffold.second_state[0]]
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
