# Harmonia metric audit — 2026-10-30

ATLAS-HARMONIA-NORM-009 [patch] investigates the current public coupling
solver at `517721292732779767ba17e77ef85ae62f198ad0`. Open PRs #75/#76
preserve old lock/manifest rescue work, and #88 owns hook rollout; none changes
the metric path. Those branches are preserved.

The unscaled sum of squares erases tiny nonzero residuals and overflows on
finite large norms. The unchanged source fails three new public-API tests:
tiny and subnormal residuals return success with zero residual, while a
large representable norm returns `NonFiniteMetric`. The fix applies one
native-precision scaled norm algorithm to both metric reductions. Acceptance
uses analytical Pythagorean witnesses, not changed convergence tolerances.

Local/hosted qualification and independent source review remain pending.
This audit does not close broader error-path or transfer coverage items.
