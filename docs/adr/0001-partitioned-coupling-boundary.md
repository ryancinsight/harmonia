# ADR 0001: Partitioned coupling boundary

- Status: accepted
- Change class: minor, architectural
- Date: 2026-07-20

## Context

Atlas domain packages repeatedly need to exchange interface state, advance
heterogeneous physics solvers on independent time grids, relax interface
updates, and terminate a fixed-point iteration. The partitioned approach
decomposes a coupled system into separately advanced partitions
([Felippa, Park, and Farhat, 2001, section 1](https://doi.org/10.1016/S0045-7825(00)00391-1)).
Waveform relaxation admits independent time discretizations; Jacobi waveform
iteration advances subproblems independently before exchanging interface
functions
([Meisrimel and Birken, 2021, sections 1 and 3](https://arxiv.org/abs/2106.13147)).

Horae already owns simulation time and const-generic subcycle ratios. Athena
Core already owns validated convergence thresholds and allocation-free
iteration observation. Duplicating either law in Harmonia would create a
second source of truth.

## Decision

Phase 0 is one `no_std + alloc` crate. It owns a two-partition synchronous
Jacobi fixed-point iteration over one time window:

1. Snapshot caller-owned states and interface guesses.
2. Restore both partition work states from the snapshots.
3. Advance each partition over the window using its const-generic Horae
   subcycle plan.
4. Export and transfer both interface states.
5. Check the unrelaxed Euclidean defect `r = ||F(x) - x||`.
6. Commit work states and `F(x)` only when Athena's policy accepts `r`.
7. Otherwise compute `x <- x + omega (F(x) - x)` and repeat.

Transfer implementations return `Cow<'a, [T]>`. Identity transfer returns the
source borrow; index transfer writes caller-owned scratch and returns that
borrow. Neither path allocates. Static transfer and relaxation policies are
zero-sized types. The complete loop is generic over the pair model, scalar,
transfer, relaxation, observer, and two const subcycle ratios; Rust
monomorphizes the operation without vtables.

The workspace owns fixed `Box<[T]>` buffers. Construction validates dimensions
once. A solve performs no allocation and accepts only slices of those validated
lengths.

## Theorems and proof obligations

### Transaction theorem

**Claim.** If `solve_window` returns an error, every caller-provided state and
interface slice equals its entry value.

**Proof.** The algorithm copies caller slices into workspace snapshots and
mutates only workspace buffers during every iteration. The only writes to
caller slices occur together in the convergence branch immediately before the
success report. Every error path returns before that branch. Therefore an
error performs no caller-visible write. The regression test compares every
slice bit-for-bit after forced nonconvergence.

### Contraction residual theorem

Let `F: W -> W` be a contraction on a closed subset of a Banach space with
factor `0 <= q < 1`, and let `x* = F(x*)`. For any `x in W`, define
`r = ||F(x) - x||`. Then:

`||x - x*|| <= r / (1 - q)`.

**Proof.** By the triangle inequality and contraction property,

`||x - x*|| <= ||x - F(x)|| + ||F(x) - F(x*)||`

`<= r + q ||x - x*||`.

Moving the final term to the left yields
`(1 - q) ||x - x*|| <= r`, and division by positive `1 - q` proves the
claim. This is the standard a-posteriori contraction estimate; compare
[Banach fixed-point theorem, Theorem 1.27](https://www.numa.uni-linz.ac.at/Teaching/LVA/2010w/NuPDE/banach.pdf).
The analytical linear-pair test independently solves the fixed point and
asserts this bound with a rounding allowance derived from machine epsilon.

### Relaxation-honesty theorem

**Claim.** A relaxation weight cannot cause false convergence.

**Proof.** The convergence predicate consumes `||F(x) - x||` before relaxation.
The scaled update `omega (F(x) - x)` is never passed to Athena. Therefore
choosing `omega` near zero cannot reduce the checked defect. A regression test
uses a tiny positive weight, a nonzero constant defect, and a tolerance between
the scaled update and raw defect; the solve must exhaust its budget.

### Subcycle endpoint invariant

Each partition receives exactly its const-generic number of positive substeps.
The first `N-1` use Horae's derived child step. The final substep is the typed
duration from the accumulated instant to the exact window endpoint. Thus no
substep starts beyond the endpoint and the final advance ends at that endpoint,
subject to the scalar arithmetic contract. Differential tests compare
specializations on systems whose exact result is subdivision-invariant.

## Rejected alternatives

- Consumer-owned coupling loops: rejected because they duplicate convergence,
  transfer, transaction, and subcycle mechanics across domain packages.
- Dynamic trait objects: rejected because the implementation set is known at
  each call site and static dispatch preserves inlining and scalar/backend
  specialization.
- Harmonia-owned time or convergence types: rejected because Horae and Athena
  are the authoritative providers.
- An extensible N-partition graph in Phase 0: rejected because no stable
  scheduling and ownership contract exists yet; speculative graph machinery
  would add indirection without a present requirement.

## Verification

- Analytical fixed-point and contraction-bound oracle.
- Property tests over contractive linear maps.
- Differential tests across `f32`/`f64` and heterogeneous subcycle ratios.
- Transaction, non-finite, dimension, transfer, and relaxation boundary tests.
- Pointer identity and `Cow::Borrowed` assertions for identity transfer.
- Type-layout assertions for zero-sized static policies.
- Allocation instrumentation proving repeated solves allocate zero times after
  workspace construction.
- Release assembly from
  `cargo rustc --test codegen_equivalence --release -- --emit=asm` contains one
  `generic_full_update` body and routes both generic and handwritten-reference
  calls to it; LLVM merged the identical implementations.
- Format, no-default-features check, Clippy with warnings denied, nextest,
  doctests, docs, example, and cargo-deny; the SemVer baseline limitation is
  recorded below.

`cargo-semver-checks` cannot establish a Phase 0 baseline: crates.io already
contains an unrelated music-theory package named `harmonia` at `0.1.0`.
Harmonia is `publish = false`; the registry package is not an API predecessor.
SemVer comparison begins from this repository's first published Git contract.
