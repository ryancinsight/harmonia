# Changelog

All notable changes to Harmonia are documented in this file.

## Unreleased

### Changed

- Follow the current provider cohort. `aequitas` moves to 0.3.1, `eunomia` to
  0.10.0, `athena-core` to 0.3.2, `horae-time` to 0.2.2, and the dev-only
  `mnemosyne-memory` to 0.10.1; the lock resolves each at its published
  revision. One git source resolves to one revision, so every edge that names
  a provider repository has to agree on a version or the resolver refuses the
  pair. `athena-core` and `eunomia` are the two that gated the set: Athena
  0.3.0 still required Eunomia 0.9, which no longer resolves against the
  current Eunomia source.
- Make relaxation a mutable pair-level contract so stateful policies can use the
  complete coupled defect and retain history across iterations without a
  consumer-owned loop or adapter.
- The identity transfer validates that caller-provided scratch storage has
  exactly the destination dimension, returning a typed `TransferError::Dimension`
  on mismatch instead of silently ignoring the scratch. The transfer contract
  documents the invariant so every boundary validates consistently. A
  value-semantic test pins the rejection.
- Align the direct Eunomia scalar dependency with the canonical `0.7.0` Git
  source (URL-only form, chases main HEAD `c65e324` carrying NEON F8
  vectorization past the v0.7.0 tag), matching athena/horae/aequitas and
  collapsing the dual-source-ID collision that broke `T: RealField`
  resolution across the ConvergencePolicy surface.

### Added

- Transactional Jacobi partitioned coupling over two independently subcycled
  partitions.
- Borrow-preserving transfer contracts, const-generic index transfer, and
  zero-sized identity and full-relaxation policies.
- Reusable coupling workspaces with allocation-free window solves.
- Analytical, property, differential, layout, and allocation evidence for the
  Phase 0 coupling laws.
