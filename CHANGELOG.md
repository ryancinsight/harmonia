# Changelog

All notable changes to Harmonia are documented in this file.

## Unreleased

### Changed

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
