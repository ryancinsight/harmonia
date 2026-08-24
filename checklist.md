# Checklist — active sprint

## gap-audit-2026-08-20 (owner: atlas-gap-audit)

Evidence-only pass at `3d6682f` (detached HEAD, four peer-modified files left
untouched). No Rust source, manifest, or CI file was changed. Items are ordered
so each step's evidence feeds the next.

- [x] Orient: `git log --oneline -8`, `git status -sb`, `git status
      --porcelain`. HEAD `3d6682f`, detached, four dirty paths
      (`.github/workflows/book-pages.yml`, `Cargo.lock`, `docs/book/book.toml`,
      `examples/coupled_decay.rs`) — an in-flight `mdbook test` wiring change,
      preserved as found.
- [x] Read declared scope: `README.md`, `CHANGELOG.md` (Unreleased),
      `docs/adr/README.md` and all three Accepted ADRs, `docs/book/SUMMARY.md`.
- [x] Measure: 1 package (no `[workspace]`), 1647 src LOC across 22 files
      (largest `src/coupling/pair/algorithm.rs` at 413), 1115 test/example LOC,
      25 test functions (24 `#[test]` + 1 `proptest!`), 8 integration test
      binaries, 1 example, 0 benches.
- [x] Conformance census: `todo!`=0, `unimplemented!`=0, TODO/FIXME/HACK=0,
      production `unwrap()`=0, `dyn `=0, `pub use ... as `=0, files>500L=0,
      type-suffixed identifiers=0, crates missing `#![deny(missing_docs)]`=0,
      junk-drawer modules=0, implementation-bearing `lib.rs`/`mod.rs`=0
      (all manifests, ≤32 lines), allow sites=1, `#[expect]` sites=7.
- [x] Coupling-coverage question: implemented — synchronous **Jacobi**
      partitioned iteration, const-generic heterogeneous subcycling
      (`FIRST_SUBSTEPS`/`SECOND_SUBSTEPS`), `FullRelaxation`,
      `FixedRelaxation<T>`, `AitkenRelaxation<T>` (Irons–Tuck componentwise
      secant, clamped), `IdentityTransfer`, `IndexTransfer<SOURCE>`.
      Declared-and-excluded rather than stubbed: Gauss-Seidel ordering, >2
      partitions, waveform interpolation, Anderson/IQN-ILS acceleration,
      distributed scheduling, conservation-aware nonmatching-mesh transfer
      (`README.md:114`, `docs/book/stack_position.md:153`). Implicit staggered
      schemes are not mentioned in any declared scope.
- [x] Conservation question: no test asserts a conserved or consistently
      transferred interface quantity; no transfer test runs inside a coupled
      solve → `ATLAS-HARMONIA-TRANSFER-004`.
- [x] Transactional question: the commit path is real — the four caller slices
      are written only in `commit` (`src/coupling/pair/algorithm.rs:295`) — but
      it is verified on one error variant of eight, no test partition can fail
      (`type Error = Infallible` throughout `tests/support/mod.rs`), and model
      state (partition internals, Aitken history) is outside the guarantee →
      `ATLAS-HARMONIA-ERRPATHS-003`, `ATLAS-HARMONIA-TXSCOPE-002`.
- [x] Consumption question: `CFDrs` consumes `AitkenRelaxation` +
      `Relaxation` in `cfd-2d` (commit `4931f85b`); no repository instantiates
      `PartitionedPair` → `ATLAS-HARMONIA-CONSUME-005`.
- [x] Cross-check README and Accepted-ADR claims against the code. Verified:
      zero-sized static policies (`tests/policies.rs:74`), allocation-free
      repeat solves (`tests/allocation.rs`), `f32`/`f64` instantiation
      (`tests/generic_scalar.rs`, `tests/aitken.rs`), subcycle differential
      (`tests/subcycling.rs`), a-posteriori contraction bound
      (`tests/theorems.rs`, `tests/properties.rs`), Aitken secant/clamp/
      small-denominator oracles (`tests/aitken.rs`). Falsified: the "no
      repository declares a dependency" claim, and ADR 0001's "every error
      path" transaction claim.
- [x] Fix unambiguous doc drift: `docs/book/stack_position.md` Consumers
      section now records the CFDrs relaxation adoption and states that
      `PartitionedPair` itself is still unconsumed.
- [x] File gaps as DoR-shaped items in `backlog.md` (8 items,
      `ATLAS-HARMONIA-REPLAY-001` … `-ALLOWATTR-008`).

### Next execution order (unclaimed)

- [ ] `ATLAS-HARMONIA-ERRPATHS-003` — cheapest, unblocks the transaction
      theorem's own claim; add the failing-partition and failing-transfer
      fixtures first.
- [ ] `ATLAS-HARMONIA-TXSCOPE-002` — decide and document the Aitken
      window-boundary semantics; needs the fixtures from -003.
- [ ] `ATLAS-HARMONIA-ADRGEN-006` and `-GATES-007` — independent hygiene, run
      in parallel with the above.
- [ ] `ATLAS-HARMONIA-TRANSFER-004` — dimension-changing coupled solve plus
      the conservation/consistency property.
- [ ] `ATLAS-HARMONIA-REPLAY-001` — ADR first (checkpoint seam vs. required
      statelessness), then the load-bearing-internal-state regression.
- [ ] `ATLAS-HARMONIA-CONSUME-005` — gated on -001.
- [ ] `ATLAS-HARMONIA-ALLOWATTR-008` — fold into whichever test-tree item
      lands first.

### Not verified by this pass

No cargo command was run: the audit was static, so no claim here asserts that
any suite compiles or passes. The most recent recorded run is ADR 0003's
verification section at the provider head prior to `3d6682f`.
