//! Allocation evidence for reusable coupling workspaces.

mod support;

use athena_core::ConvergencePolicy;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

use support::{LastObserver, LinearPartition, Scaffold, linear_pair, solve};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

#[test]
fn repeated_window_solves_allocate_nothing_after_workspace_construction() {
    let mut pair = linear_pair::<f64, 2, 3>(
        LinearPartition {
            source: 1.0,
            gain: 0.1,
        },
        LinearPartition {
            source: -0.5,
            gain: -0.2,
        },
    );
    let policy = ConvergencePolicy::new(1.0e-10, 1.0e-10, 32).expect("invariant: valid policy");
    let mut scaffold = Scaffold::new(0.0, 0.0);

    let region = Region::new(ALLOCATOR);
    for _ in 0..16 {
        solve(
            &mut pair,
            0.25,
            &policy,
            &mut scaffold,
            &mut LastObserver::default(),
        )
        .expect("contractive pair converges");
    }
    let change = region.change();

    assert_eq!(change.allocations, 0);
    assert_eq!(change.reallocations, 0);
    assert_eq!(change.deallocations, 0);
}
