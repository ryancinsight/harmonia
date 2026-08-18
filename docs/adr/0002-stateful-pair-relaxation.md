# ADR 0002: Stateful pair relaxation boundary

- Status: Accepted
- Change class: minor, architectural
- Date: 2026-08-18

## Context

Harmonia's Phase 0 loop owns the fixed-point defect over both transferred
interfaces, but its original `Relaxation<T>` contract accepted one interface at
a time through an immutable policy. That shape prevents a real stateful policy
from using the complete coupled defect or retaining history across iterations.
It also makes the order of two separate calls part of an undocumented policy
contract. CFDrs currently has a consumer-owned stateful resistance mixer; the
provider boundary must be able to express that class of policy before a direct
migration can delete the consumer loop.

## Decision

`Relaxation<T>` exposes one mutable `update_pair` operation with both current
interfaces and both candidates. `PairModel` exposes the policy through
`relaxation_mut`, and `PartitionedPair` invokes the operation once for each
non-converged iteration. The built-in fixed and full policies retain their
existing value semantics and validate both dimensions and all updated values.

Non-finite indices are addressed in the concatenation of the first and second
interfaces. An implementation that returns an error must not partially update
either interface; this is the policy boundary needed by the transactional
coupling theorem in ADR 0001.

This ADR promotes the provider seam only. It does not duplicate an Anderson or
Aitken algorithm in Harmonia: iterative solver policy remains owned by its
existing provider, and its direct integration is a subsequent co-evolution
change once that provider implements this contract.

## Rejected alternatives

- Keep two immutable calls: rejected because stateful policies would observe
  incomplete state and call order would be an accidental contract.
- Add a consumer-owned wrapper around the old trait: rejected because it keeps
  the integration gap and violates provider-first ownership.
- Copy CFDrs' acceleration algorithm into Harmonia: rejected because it would
  create a second iterative-solver source of truth.

## Verification

- `cargo check --locked --all-targets` passes for the standalone Harmonia
  manifest with the pinned MSVC toolchain.
- `cargo nextest run --locked` passes all 16 tests, including the stateful
  policy regression that observes one retained update across two iterations.
- The full/fixed policy tests retain value and boundary coverage, and the
  allocation and generic-scalar suites remain green.
