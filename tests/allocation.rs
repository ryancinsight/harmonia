//! Allocation evidence for reusable coupling workspaces.
//!
//! The measurement window counts allocations made by the calling thread only.
//! A process-wide counter is invalid here: libtest runs the test body on a
//! spawned thread while its main thread keeps inserting the running test into
//! its bookkeeping collections, and parallel tests allocate concurrently, so a
//! process-wide window occasionally absorbs allocations unrelated to
//! the coupling path.

mod support;

use std::alloc::System;

use athena_core::ConvergencePolicy;
use mnemosyne::counting::{AllocationDelta, CountingAllocator, measure};

use support::{LastObserver, LinearPartition, Scaffold, linear_pair, solve};

#[global_allocator]
static ALLOCATOR: CountingAllocator<System> = CountingAllocator::new(System);

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

    let ((), delta) = measure(|| {
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
    });

    assert_eq!(delta, AllocationDelta::default());
}
