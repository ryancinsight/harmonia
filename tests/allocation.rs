//! Allocation evidence for reusable coupling workspaces.

mod support;

use athena_core::{ConvergencePolicy, NoObserver};
use harmonia::{FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace, PartitionedPair};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

use support::{LinearPartition, instant, window};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

#[test]
fn repeated_window_solves_allocate_nothing_after_workspace_construction() {
    let model = PairComponents::new(
        LinearPartition {
            source: 1.0_f64,
            gain: 0.1,
        },
        LinearPartition {
            source: -0.5_f64,
            gain: -0.2,
        },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 2, 3>::new(model, workspace).expect("valid subcycles");
    let policy = ConvergencePolicy::new(1.0e-10, 1.0e-10, 32).expect("invariant: valid policy");
    let start = instant();
    let step = window(0.25);
    let mut first_state = [0.0];
    let mut second_state = [0.0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];

    let region = Region::new(ALLOCATOR);
    for _ in 0..16 {
        pair.solve_window(
            start,
            step,
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        )
        .expect("contractive pair converges");
    }
    let change = region.change();

    assert_eq!(change.allocations, 0);
    assert_eq!(change.reallocations, 0);
    assert_eq!(change.deallocations, 0);
}
