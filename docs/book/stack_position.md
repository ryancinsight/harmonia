# 3. Position in the Stack

Harmonia is one repository in the [Atlas](https://github.com/ryancinsight/atlas)
multiphysics stack. This chapter says what it owns, what it delegates, and why
the boundaries fall where they do — the information you need to decide whether a
change belongs here or in one of its providers.

## The layering

Atlas layers one way, foundation to integrator; a dependency pointing the other
way is an architectural defect rather than a preference.

```text
eunomia        scalar law: RealField, NumericElement, native-precision contracts
   ↓
aequitas       physical quantities: type-level SI dimensions over those scalars
   ↓
horae          typed simulation time, step sizes, const-generic subcycle ratios
athena-core    convergence policy, iteration observation, termination vocabulary
   ↓
harmonia       partitioned coupling mechanics   ←  you are here
   ↓
domain crates  discretisation, equations, material law, physics solvers
   ↓
integrators    CFDrs, helios, kwavers
```

Harmonia sits above the time and convergence providers and below anything that
knows what the equations mean. That boundary is exactly Chapter 1's contract:
the coupling loop needs a state vector it can advance and an interface vector it
can exchange, and nothing else. Everything below supplies the vocabulary those
are expressed in. Everything above decides what they mean.

The dependency closure is small and deliberately so. Harmonia's normal
dependencies are `athena-core`, `eunomia`, and `horae`, with `aequitas` arriving
transitively through Horae's typed time. There is no array provider and no
device provider in the graph: Harmonia never touches a field, so it never needs
`leto`, and it never dispatches a kernel, so it never needs `hephaestus`. It
depends on `athena-core` rather than the `athena` facade for the same reason —
the facade brings CPU and accelerator backends that a coupling loop has no use
for.

The crate is `no_std` with `alloc`, forbids `unsafe_code`, and denies missing
documentation. A partition's state and interface arrive as `&mut [T]` and
`&[T]`; storage, layout, and residency belong to whoever owns the fields.

## What Harmonia owns

- the `Partition<T>` contract and the typed `Substep<T>` handed to each advance;
- the transactional two-partition Jacobi fixed-point loop, `PartitionedPair`,
  and its all-or-nothing commit;
- the `Transfer<T>` contract and its `IdentityTransfer` and
  `IndexTransfer<SOURCE>` policies, returning `Cow<'a, [T]>` so a transfer
  borrows rather than allocates;
- the `Relaxation<T>` contract and the `FullRelaxation`, `FixedRelaxation<T>`,
  and `AitkenRelaxation<T>` policies of Chapter 2;
- `PairWorkspace`, which allocates once and validates every dimension against
  the pair model before a solve can be built;
- the outcome vocabulary: `CouplingReport` on success and `CouplingError` on
  every failure mode, with both partitions' error types preserved rather than
  collapsed to a string.

## What Harmonia does not own

**Time** is Horae's. `Instant<T>`, `StepSize<T>`, and the const-generic
`SubcyclePlan<RATIO>` are Horae types; Harmonia's `Substep<T>` wraps them and
adds an index and a count. Harmonia defines no duration type, derives no child
step of its own, and validates no ratio: the substep loop asks
`SubcyclePlan::<N>::new()` and `child_step` for both, so a change to the subcycle
law lands in one place.

**Convergence** is Athena's. `ConvergencePolicy` supplies the
absolute-plus-relative threshold and the iteration budget; `IterationObserver`
and `IterationState` carry the trace. Harmonia contributes the two norms — the
defect and the candidate scale — and consumes the verdict. It defines no
tolerance and no stopping rule.

**Scalars** are Eunomia's. `T: RealField` is `eunomia::RealField`, arithmetic and
accumulation happen in `T` itself, and the algorithm is instantiated across every
supported scalar rather than written at one precision.

**Physics** is yours. The `Partition<T>` implementation contains the equations,
the discretisation, the time integrator, and the linear algebra. Harmonia never
sees a mesh, a field, a material law, a linear solver, an accelerator, or a
runtime.

The last exclusion is what shapes the API. A coupling loop that knew it was
coupling a fluid to a structure could exploit the added-mass structure of
Chapter 2 and build a preconditioner from it. One that does not know can couple
any two partitions that can be advanced and can export an interface — and pays
for that with a caller-implemented trait and with the relaxation policy having
no information about where the bad eigenvalue comes from.

## Static composition

`PairModel<T>` bundles the five constituents — two partitions, two directed
transfers, one relaxation policy — as associated types, and `PairComponents` is
the concrete bundle. The bundle exists so `PartitionedPair` carries one model
parameter instead of five independent generic parameters chained through every
signature.

Nothing in the loop is dynamically dispatched. The pair model, the scalar, both
transfers, the relaxation policy, the observer, and both substep counts are all
resolved at compile time, so a solve monomorphises to straight-line code over
the caller's own types with no vtable on the path. The transfer and relaxation
policies that carry no data are zero-sized, and the test suite asserts that
rather than assuming it. `tests/codegen_equivalence.rs` pins the stronger claim
for relaxation: a generic zero-sized update and a handwritten concrete reference
are asserted bit-identical over finite, infinite, and NaN candidates, and the
release assembly recorded in ADR 0001 shows LLVM merging the two into one body.

The two substep counts are const generic parameters, so a pair advancing two
substeps against three is a distinct type from one advancing one against one.
That is the intended granularity: the counts are a property of the coupled
configuration, fixed for the duration of a run.

## Allocation and reuse

`PairWorkspace::for_model` performs every allocation required by the coupling
workspace. It reads the six dimensions from the two partitions, rejects any
that is zero, checks each directed transfer's destination dimension against the
receiving partition's input, and allocates twelve fixed boxed slices. Static
relaxation policies then write only into those buffers and into the caller's
four slices. `AitkenRelaxation<T>` separately allocates its retained history
and scratch vectors on first use, then reuses their capacity.

Repeated solves with static policies therefore allocate nothing, which matters
because a simulation calls `solve_window` once per coupling window for the
length of the run. The claim is instrumented rather than asserted: an
allocation-counting global allocator wraps sixteen consecutive solves and
asserts zero allocations, zero reallocations, and zero deallocations. Stateful
Aitken history is an explicit retained allocation rather than part of that
static-policy claim.

## Crate layout

Harmonia is a single crate. Its modules follow the vocabulary of the previous
two chapters.

| Module | Contents |
| --- | --- |
| `partition` | the `Partition<T>` contract and `Substep<T>` |
| `transfer` | the `Transfer<T>` contract, identity and const-index policies, `TransferError` |
| `relaxation` | the `Relaxation<T>` contract, static and stateful policies, and their errors |
| `coupling` | `PartitionedPair`, `PairModel`, `PairComponents`, `PairWorkspace`, `CouplingReport`, `CouplingError` |

## The Phase 0 boundary

Harmonia's current scope is one coupling family, and the exclusions are explicit
rather than stubbed. There is no feature flag hiding an unimplemented path and no
`todo!()` behind a trait method.

| Excluded | Why |
| --- | --- |
| interface waveform interpolation | the interface is a zeroth-order hold across a window (Chapter 1) |
| more than two partitions | no stable scheduling and ownership contract for a partition graph exists yet |
| Gauss-Seidel ordering | Phase 0 evaluates both partitions from the same iterate |
| Anderson-style quasi-Newton acceleration | remains Leto-owned; Harmonia's Aitken policy is the supported dynamic relaxation boundary (Chapter 2) |
| distributed scheduling | execution placement belongs to its own provider |
| conservation-aware nonmatching-mesh transfer | requires a mesh, which Harmonia does not have |

Each of those is a present-requirement question, not a permanent one. The
decision, the alternatives weighed against it, and the proof obligations
discharged for what does exist are recorded in
`docs/adr/0001-partitioned-coupling-boundary.md`. The stack-level promotion of
coupling ownership into this repository is recorded in Atlas
[ADR 0023](https://github.com/ryancinsight/atlas/blob/main/docs/adr/0023-harmonia-coupling-promotion.md).

## Consumers

Atlas's dependency sketch places Harmonia between the Horae and Athena providers
and the three integrators, `CFDrs`, `helios`, and `kwavers`. One migration has
run: `CFDrs` declares Harmonia as a Git dependency and its `cfd-2d` network
coupling uses `AitkenRelaxation<T>` through the `Relaxation<T>` contract. The
crate is `publish = false`, so consumers take it by Git source.

The coupling loop itself has no consumer yet. `PartitionedPair` and the
`Partition<T>` contract are still unexercised outside this repository's own
test suite, and that matters for reading the API. Every contract here was
derived from the coupling mechanics the integrators were each about to
reimplement, and the first `Partition` migration is the test of whether that
contract is the right shape. A trait obligation that turns out to be
unsatisfiable by a real physics solver is a defect in this crate, not in the
consumer.

## Adding a policy

A new interface transfer implements `Transfer<T>`: report the destination
dimension for a given source dimension, and fill or borrow the caller's scratch.
A new relaxation implements `Relaxation<T>`: update both current interfaces
toward their candidates in one `update_pair` call. Stateful policies can retain
history between iterations without changing the coupling loop, and all policies
compose into a `PairComponents` bundle at the call site.

A new *coupling family* — Gauss-Seidel ordering or three or more partitions — is
a change to the loop, and is a decision recorded in an ADR before it is a change
to the code. A dynamic relaxation policy is a policy-level change when it fits
the existing pair contract.
