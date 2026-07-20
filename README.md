# Harmonia

Harmonia is the Atlas owner for multiphysics coupling mechanics. Phase 0
provides transactional, synchronous Jacobi partitioned iteration across one
time window. It composes:

- Horae for typed simulation time and const-generic subcycle ratios;
- Athena Core for convergence policy and iteration observation;
- Eunomia for native-precision real scalar contracts.

Physics models remain in their domain packages. Harmonia receives borrowed
partition state and interface slices; it owns no mesh, field, material law,
linear solver, accelerator, or runtime.

## Phase 0 contract

`PartitionedPair<M, T, FIRST_SUBSTEPS, SECOND_SUBSTEPS>` advances two
partitions from the same window-start snapshot. Each fixed-point iteration
replays both partitions independently, transfers their exported interface
values, checks the raw fixed-point defect, and applies relaxation only when
another iteration is required.

Caller states and interface guesses change only on convergence. Every error,
including iteration-budget exhaustion, leaves them unchanged. Workspace
allocation occurs during construction; repeated solves with Harmonia's
borrowed transfer policies reuse fixed boxed slices without allocation.

```rust
use aequitas::systems::si::quantities::Time;
use athena_core::{ConvergencePolicy, NoObserver};
use harmonia::{
    FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace, Partition,
    PartitionedPair, Substep,
};
use horae::time::{Instant, StepSize};

#[derive(Clone, Copy)]
struct Driven {
    gain: f64,
}

impl Partition<f64> for Driven {
    type Error = core::convert::Infallible;

    fn state_dimension(&self) -> usize { 1 }
    fn input_dimension(&self) -> usize { 1 }
    fn output_dimension(&self) -> usize { 1 }

    fn advance(
        &mut self,
        step: Substep<f64>,
        state: &mut [f64],
        input: &[f64],
    ) -> Result<(), Self::Error> {
        state[0] += step.size().as_time().as_base() * (1.0 + self.gain * input[0]);
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        output.copy_from_slice(state);
        Ok(())
    }
}

let model = PairComponents::new(
    Driven { gain: 0.2 },
    Driven { gain: 0.1 },
    IdentityTransfer,
    IdentityTransfer,
    FullRelaxation,
);
let workspace = PairWorkspace::for_model(&model).unwrap();
let mut coupling = PartitionedPair::<_, f64, 2, 3>::new(model, workspace).unwrap();
let mut first = [0.0];
let mut second = [0.0];
let mut first_input = [0.0];
let mut second_input = [0.0];
let start = Instant::new(Time::from_base(0.0)).unwrap();
let window = StepSize::new(Time::from_base(0.25)).unwrap();
let policy = ConvergencePolicy::new(1.0e-12, 1.0e-12, 16).unwrap();

let report = coupling
    .solve_window(
        start,
        window,
        &mut first,
        &mut second,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut NoObserver,
    )
    .unwrap();

assert!(report.residual_norm <= report.threshold);
```

## Mathematical evidence

For a contraction `F` with factor `q < 1`, a computed interface iterate `x`
and fixed point `x*` satisfy

`||x - x*|| <= ||F(x) - x|| / (1 - q)`.

The proof and its assumptions are recorded in
[ADR 0001](docs/adr/0001-partitioned-coupling-boundary.md). The test suite
checks this bound against an independently solved linear coupled system,
instantiates the generic algorithm at `f32` and `f64`, compares heterogeneous
subcycle specializations, proves transactional failure behavior, and measures
zero allocations after workspace construction.

## Scope boundary

Phase 0 intentionally excludes waveform interpolation, more than two
partitions, Gauss-Seidel ordering, quasi-Newton acceleration, distributed
scheduling, and conservation-aware nonmatching-mesh transfer. Those
capabilities require additional present contracts; they are not hidden behind
stubs or feature flags.
