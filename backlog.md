# Backlog — harmonia

Gap-audit baseline recorded 2026-08-20 at `3d6682f` (detached). Every item below
is DoR-shaped: outcome, scope/non-goals, acceptance oracle, dependencies,
risk/change class, status. Execution ordering lives in `checklist.md`.

Measured baseline at that revision: 1 package, 1647 src LOC, 25 test functions,
3 ADRs, 5 book pages; zero `todo!`/`unimplemented!`/TODO markers, zero
production `unwrap()`, zero `dyn` sites, zero files over 500 lines, zero
type-suffixed identifiers, zero re-export shims.

## ATLAS-HARMONIA-REPLAY-001 — Partition replay is not state-complete [minor][arch] — todo

- Outcome: a fixed-point iteration replays the *whole* partition from the
  window-start snapshot, not only the caller's state slice, so the iterated map
  `F` is the same map on every iteration for a partition that carries internal
  state.
- Evidence of the gap: `evaluate` restores `first_work`/`second_work` from the
  snapshots (`src/coupling/pair/algorithm.rs:197`) and then calls
  `advance_window` on `self.model.first_mut()` (`:202`), but
  `Partition::advance` takes `&mut self` and the contract explicitly permits
  internal workspaces (`src/partition/contract.rs:8`). Nothing restores that
  internal state between iterations, so a multistep integrator, a retained
  sub-iteration cache, or an internal accumulator makes iteration `k+1` a
  different map than iteration `k`. ADR 0001's contraction argument
  (`docs/adr/0001-partitioned-coupling-boundary.md:65`) assumes one fixed `F`.
- Scope: the `Partition<T>` contract, the replay step of `PartitionedPair`, an
  ADR recording the chosen seam, and the tests below.
- Non-goals: waveform interpolation, Gauss-Seidel ordering, more than two
  partitions, or any change to Horae's subcycle law.
- Acceptance oracle: a test partition whose internal state is load-bearing
  (an internal step counter that changes `advance`'s result) converges to the
  same interface fixed point as the equivalent stateless partition, to a
  derived tolerance; the same test fails against today's loop. Either the trait
  gains an explicit checkpoint/restore obligation that the loop calls, or the
  contract documents statelessness as a *required* invariant and the loop
  cannot silently violate it.
- Dependencies: none. Decision must precede ATLAS-HARMONIA-CONSUME-005, since
  a real solver is the class of partition that carries internal state.
- Risk/change class: `[correctness]`, `[minor]` if the seam is additive on the
  trait, `[major]` if a required method lands; `[arch]` either way. Effort M.

## ATLAS-HARMONIA-TXSCOPE-002 — Transaction scope excludes model state [patch] — todo

- Outcome: the transaction guarantee states exactly what it covers, and a
  failed window cannot silently seed the next one.
- Evidence of the gap: ADR 0001's transaction theorem
  (`docs/adr/0001-partitioned-coupling-boundary.md:54`) and `README.md:23`
  say every error leaves caller state unchanged — true for the four caller
  slices, which are written only in `commit`
  (`src/coupling/pair/algorithm.rs:295`). But `solve_window` mutates the model
  on every error path: partitions advance through `first_mut()`, and the
  relaxation policy is updated once per non-converged iteration
  (`:285`). `AitkenRelaxation` commits history inside `update_pair`
  (`src/relaxation/aitken.rs:161`), so a window that exhausts its budget leaves
  a residual/factor history from a discarded iterate, and the next
  `solve_window` computes its first secant factor across that discontinuity.
  `AitkenRelaxation` exposes no reset.
- Scope: `AitkenRelaxation` (a reset or window-boundary contract), the
  `Relaxation<T>` and `Partition<T>` doc contracts, ADR 0001's theorem
  statement, `README.md`, and `docs/book/relaxation.md`.
- Non-goals: making the model itself transactional (that is
  ATLAS-HARMONIA-REPLAY-001's decision).
- Acceptance oracle: a value-semantic test that runs a window to
  `NotConverged` with `AitkenRelaxation`, then runs a fresh window, and asserts
  the second window's first update equals the documented contract (either a
  fresh unit factor after an explicit reset, or the retained factor if
  retention is the decided semantics). The theorem text names caller slices and
  model state separately.
- Dependencies: none.
- Risk/change class: `[correctness]`/`[docs]`, `[patch]` for the doc scoping,
  `[minor]` if a `reset` method is added. Effort S.

## ATLAS-HARMONIA-ERRPATHS-003 — Seven of eight error variants unverified [patch] — todo

- Outcome: the transaction theorem is verified on every reachable error path,
  not one.
- Evidence of the gap: `nonconvergence_is_transactional`
  (`tests/theorems.rs:71`) covers `CouplingError::NotConverged` only. Every
  test partition declares `type Error = Infallible`
  (`tests/support/mod.rs:21`, `:61`, `:99`), so `CouplingError::First` and
  `::Second` (`src/coupling/pair/error.rs:90`) are never constructed, and
  `Dimension`, `Time`, `Subcycle`, `Transfer`, and `NonFiniteMetric` have no
  test at all — while ADR 0001 claims the theorem for *every* error path.
- Scope: `tests/theorems.rs`, `tests/support/mod.rs` (a failing partition
  fixture and a failing transfer fixture), no source change expected.
- Non-goals: new error variants.
- Acceptance oracle: one test per reachable variant asserting the variant is
  returned *and* all four caller slices are bit-identical to entry; a
  `FailingPartition` returning its typed error on a chosen substep index
  exercises `First`/`Second`; a mismatched caller slice exercises `Dimension`;
  a non-finite partition output exercises `NonFiniteMetric`.
- Dependencies: none.
- Risk/change class: `[verification]`, `[patch]`. Effort S.

## ATLAS-HARMONIA-TRANSFER-004 — Non-identity transfer never runs inside the loop [patch] — todo

- Outcome: the scratch-backed transfer path is exercised end to end, and
  interface transfer carries a conservation or consistency oracle.
- Evidence of the gap: `IndexTransfer` is tested only standalone
  (`tests/policies.rs:46`); every coupling test uses `IdentityTransfer`, which
  returns a borrow of the source and never writes
  `workspace.first_transfer`/`second_transfer`
  (`src/coupling/pair/algorithm.rs:251`). The dimension-changing branch of
  `export_and_transfer` and the `copy_from_slice` from a scratch-backed `Cow`
  therefore have no coupled-solve coverage. No test asserts any transferred
  quantity is conserved or consistently reproduced.
- Scope: `tests/`, plus whatever `Transfer<T>` doc obligation the conservation
  statement needs. Conservation-aware *nonmatching-mesh* transfer stays out of
  scope: it needs a mesh, which Harmonia does not own
  (`docs/book/stack_position.md:160`).
- Non-goals: adding a mesh, an interpolation policy, or a second transfer
  family.
- Acceptance oracle: a coupled solve where the two partitions have different
  interface dimensions and a scratch-backed transfer bridges them, converging
  to an independently computed exact interface within a derived bound; plus a
  transfer-level property that a conservative transfer preserves the summed
  quantity to `n * eps` and a consistent one preserves a constant field
  exactly.
- Dependencies: none.
- Risk/change class: `[verification]`, `[patch]`. Effort M.

## ATLAS-HARMONIA-CONSUME-005 — Coupling loop has no consumer [minor] — todo

- Outcome: at least one integrator instantiates `PartitionedPair` against a
  real physics partition, so the `Partition<T>` contract is validated by use
  rather than by fixtures.
- Evidence of the gap: `CFDrs` declares Harmonia
  (`repos/CFDrs/Cargo.toml:113`, commit `4931f85b`) but imports only
  `AitkenRelaxation` and `Relaxation`
  (`repos/CFDrs/crates/cfd-2d/src/network/coupled.rs:18`). No repository
  instantiates `PartitionedPair`, `PairComponents`, or `Partition<T>`. The
  crate's own book names this as the open risk
  (`docs/book/stack_position.md:177`).
- Scope: Harmonia-side only — the contract adjustments a first real migration
  demands, and the ADR recording them. The consumer change is that repository's
  item.
- Non-goals: editing any consumer repository from here.
- Acceptance oracle: a named consumer builds a `PartitionedPair` over its own
  solver, its contract test pins the interface semantics it relies on, and any
  contract change lands here with an ADR revision.
- Dependencies: ATLAS-HARMONIA-REPLAY-001 (a real solver is stateful).
- Risk/change class: `[arch]`, `[minor]`. Effort L.

## ATLAS-HARMONIA-ADRGEN-006 — ADR index names a generator that does not exist [patch] — done 2026-09-02

- Outcome: the ADR index claim matches the repository, and the check runs in
  CI.
- Evidence of the gap: `docs/adr/README.md:3` says the file is generated by
  `scripts/adr-index.py` and must not be hand-edited, with `generate` and
  `check` subcommands. There is no `scripts/` directory in the repository and
  no CI step invokes either subcommand (`.github/workflows/ci.yml`).
- Scope: either commit the generator and wire `check` into CI, or delete the
  generated-by comment and maintain the index by hand as a same-change doc-sync
  obligation. Prefer the generator: three ADRs already drift-prone by hand.
- Non-goals: ADR content changes.
- Acceptance oracle: `check` fails on a hand-added ADR that is missing from the
  index, and passes at HEAD; the CI job runs it.
- Dependencies: none.
- Risk/change class: `[pm-hygiene]`/`[docs]`, `[patch]`. Effort S.
- **Closed 2026-09-02 on the merged tree.** Delivered by PR #12 and #14: the
  generator is atlas's, called through the shared reusable workflow rather
  than copied here (`.github/workflows/adr-index.yml` → `adr-index-guard.yml`
  with `strict: true`), and `docs/adr/README.md` names it. The item's
  preferred branch — commit a generator — was the wrong one: atlas owns the
  single generator for the fleet, so adopting it is what keeps the index from
  drifting per repo.

## ATLAS-HARMONIA-GATES-007 — CI gate floor incomplete [patch] — done 2026-09-02

- Outcome: gates run against the committed lockfile, verify the declared MSRV,
  and cover public-surface compatibility.
- Evidence of the gap: no cargo invocation in `.github/workflows/ci.yml:26-38`
  passes `--locked`, so CI does not test the committed `Cargo.lock`;
  `Cargo.toml:5` declares `rust-version = "1.95"` while
  `rust-toolchain.toml:2` pins `1.97.0` and no job builds at the 1.95 floor, so
  the MSRV claim is unverified; there is no dependency cache restore, and no
  `cargo-semver-checks` step despite a fully public API surface.
- Scope: `.github/workflows/ci.yml` only.
- Non-goals: adding feature flags, or a mutation/coverage suite (separate
  item if wanted).
- Acceptance oracle: every cargo step carries `--locked`; an `msrv` job builds
  the crate on 1.95 and fails if the floor is wrong; `cargo-semver-checks`
  runs on pull requests touching `src/`; job wall clock stays within the
  five-minute verification target with the cache restored.
- Dependencies: none.
- Risk/change class: `[verification]`, `[patch]`. Effort S.
- **Delivered 2026-09-02.** Every resolving cargo step in `verify` carries
  `--locked` (`cargo fmt` does not resolve, so it does not take the flag); a
  `msrv` job builds `--locked --all-features --all-targets` at the declared
  1.95 floor; the shared atlas SemVer gate runs informationally on pull
  requests; `Swatinem/rust-cache` restores on every branch and saves only from
  `main`, so a pull request cannot poison the shared entry.
- **The MSRV job requests its floor through `RUSTUP_TOOLCHAIN`, not the setup
  action**, because the committed `rust-toolchain.toml` pin (1.97.0) outranks
  what the action selects — the obvious spelling would have silently re-tested
  1.97.0 and left the claim unverified. The job prints `rustc --version` and
  fails if it is not the floor, so the check cannot pass vacuously.
- **Follow-up 2026-09-02 (review finding).** `Swatinem/rust-cache` builds its
  key with `cargo metadata`, run without `--locked`; on a lockfile that has
  drifted from the manifests cargo rewrites it in that step, and every
  `--locked` step after it would verify a lockfile that is not the committed
  one — the gate passing for the wrong reason. Each cache step is now followed
  by `git diff --exit-code -- Cargo.lock`, so the assumption the `--locked`
  flags rest on is asserted rather than trusted.
- Evidence: the floor was verified locally before the job was written —
  `RUSTUP_TOOLCHAIN=1.95.0 cargo check --locked --all-features --all-targets`
  compiles harmonia and its first-party graph clean, so `rust-version = 1.95`
  is a true claim rather than an aspiration.

## ATLAS-HARMONIA-ALLOWATTR-008 — Test-harness blanket allow [patch] — todo

- Outcome: the crate's own "no blanket suppressions" floor holds in the test
  tree too.
- Evidence of the gap: `tests/support/mod.rs:1` carries
  `#![allow(dead_code)]` — the only allow site in the repository — covering the
  whole shared fixture module. `Cargo.toml`'s lint table does not enable
  `clippy::allow_attributes`, so nothing drives it toward `#[expect]`.
- Scope: `tests/support/mod.rs` and the `[lints.clippy]` table.
- Non-goals: restructuring the fixture module.
- Acceptance oracle: `clippy::allow_attributes` denied workspace-wide and the
  suppression replaced by per-item `#[expect(dead_code, reason = "...")]`, or
  the unused fixtures deleted; `cargo clippy --all-targets -- -D warnings`
  stays green.
- Dependencies: none.
- Risk/change class: `[pm-hygiene]`, `[patch]`. Effort S.
