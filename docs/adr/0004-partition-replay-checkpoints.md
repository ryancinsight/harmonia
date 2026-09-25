# ADR 0004: Partition replay checkpoints

- Status: Accepted
- Change class: major, architectural
- Date: 2026-09-24
- Board item: `ATLAS-HARMONIA-REPLAY-001`

## Context

`PartitionedPair` restores caller-owned state slices before each fixed-point
evaluation, while `Partition::advance` mutates the partition itself. The trait
permits internal workspaces but exposes no way to return them to their
window-start values. A multistep integrator, retained sub-iteration cache, or
internal accumulator therefore changes the partition map between iterations.
ADR 0001's contraction result applies only when every iteration evaluates the
same map.

Real solver partitions carry internal numerical state, so requiring partitions
to be stateless would exclude the integration class this crate exists to
coordinate.

## Decision

`Partition<T>` requires an associated `Checkpoint` type and two operations:
`checkpoint` captures owned replay state, and `restore` reinstates it. A
checkpoint contains every internally owned value whose mutation during
`advance` or `export` can affect a later `advance` or `export` result. It need
not duplicate immutable configuration.

`PartitionedPair::solve_window` captures one checkpoint from each partition
after validating and copying the four caller slices. Before every fixed-point
evaluation, including the first, it restores both partitions and both caller
state work buffers to their window-start values. Each iteration therefore
evaluates one fixed map from interface guesses to interface candidates.

Checkpoint creation and restoration are infallible. The checkpoint was
produced by the same partition instance, so inability to restore it is an
implementation invariant violation rather than an input-dependent failure.
The trait provides no default checkpoint or restore operation: every
implementation must state its replay model, and the coupling loop cannot omit
the required restoration call.

The existing transaction boundary remains unchanged. Caller-provided slices
are written only on convergence. On success the partitions retain the final
accepted evaluation state; on failure they retain the state reached on the
failing or final evaluation. Model-wide rollback remains outside this decision.

## Migration

This is a breaking trait change. Every `Partition<T>` implementation adds
`type Checkpoint`. A stateless partition uses `()` and implements both methods
explicitly. A stateful partition uses an owned value containing all mutable
algorithm state observed by replay. Cloning the partition's internal state is
valid when that state is the replay boundary; borrowed checkpoints are not.

The in-repository implementations and examples migrate in this change.
`ares-coupling::StructuralPartition` is the only external implementation found
in the Atlas checkout and must add the same contract in its own repository.
CFDrs consumes Harmonia's relaxation API but does not implement `Partition`.

## Rejected alternatives

- Require stateless partitions: rejected because real time integrators retain
  history and workspaces as part of their numerical state.
- Give checkpoint methods no-op defaults: rejected because an omitted stateful
  implementation would compile and reproduce the defect.
- Clone the entire pair model in Harmonia: rejected because transfer and
  relaxation policies have separate lifecycle contracts, and partitions own
  the representation of their numerical replay state.
- Store type-erased checkpoints: rejected because associated types retain
  static dispatch and exact ownership without allocation or vtables.

## Verification

- A load-bearing internal step counter changes `advance` output unless restored.
- The test uses window `1/2`, gains `0` and `1/8`, sources `1/2` and `-1/4`,
  states `1/4` and `-1/2`, and counters `3` and `6`. With zero first gain, each
  fixed point takes two updates. Every map intermediate is a dyadic with
  denominator at most 64 and magnitude at most `5/2`, so `(-53/64, 5/4)` and
  `(-37/64, 5/2)` are exact in `f64`. Their third evaluations subtract equal
  dyadics for zero residual; earlier norm rounding does not feed full
  relaxation. The two windows therefore require no empirical tolerance.
- A pre-change run of that regression fails because successive iterations use
  increasing internal counter values.
- Existing transaction, subcycle, scalar, allocation, documentation, and
  SemVer gates cover the complete migrated surface.
