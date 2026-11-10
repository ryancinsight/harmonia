# ADR 0005: Harmonia release scope for 1.0

- Status: Accepted
- Date: 2026-11-09
- Decision class: [arch]
- Revision: 2026-11-09 - Initial 1.0 scope and milestone ladder.

## Context

Harmonia provides Atlas partitioned multiphysics coupling orchestration as a
`no_std + alloc` crate (`publish = false`, `0.1.4`). The backlog has open
verification and consumer-integration items but no milestone ladder or
release-scope record. Sprint policy requires a release-scope ADR and a
runnable milestone ladder before 1.0 planning.

## Decision

Harmonia targets `1.0.0` through successive pre-1.0 milestones recorded in
`backlog.md`. Each milestone closes only when its `exit:` checks pass at a
fetched default-branch commit.

Planned milestones (ladder is authoritative):
- `M-contract-proof` (`0.2.0`): coupling contract verification complete
  (transaction error-path coverage and transfer oracles).
- `M-consumer-validation` (`0.3.0`): at least one real consumer validates the
  `Partition<T>` contract through `PartitionedPair`.
- `M-1.0` (`1.0.0`): SemVer/publish dry run readiness, documentation builds,
  and no open correctness items.

## Scope

In scope for 1.0: partitioned pair coupling contract, workspace/transfer
transaction guarantees, relaxation policies as specified by accepted ADRs, and
consumer-validated partition semantics.

Out of scope for 1.0 (veto points / deferred): conservation-aware nonmatching
mesh transfer (requires mesh ownership Harmonia does not have), additional
coupling topologies beyond the current two-partition model unless backed by a
new ADR and consumer need, and publication enablement beyond dry-run/readiness
checks while `publish = false`.

## Alternatives considered

- **Single jump to 1.0:** rejected; open verification gaps (ERRPATHS-003,
  TRANSFER-004) and missing consumer validation leave the contract
  insufficiently evidenced.
- **Feature-first growth:** rejected; verification and consumer proof close
  known contract risk before expansion.

## Consequences

- Backlog items carry `milestone:` assignments; the ladder orders closure.
- The 1.0 milestone blocks on release authority via the filing-sweep `blocked`
  release item when its exit checks pass.
- Any removal/weakening of exit checks requires an ADR revision and judge
  verdict per sprint policy.

## Overturning evidence

- A consumer migration reveals `Partition<T>` contract changes requiring a
  different milestone split.
- Performance or safety evidence requires a bounded contract break recorded as
  `[major]` with migration plan.
