# Harmonia metric audit — 2026-10-30

The metric-range repair [patch] investigates the current public coupling
solver at `517721292732779767ba17e77ef85ae62f198ad0`. Open PRs #75/#76
preserve old lock/manifest rescue work, and #88 owns hook rollout; none changes
the metric path. Those branches are preserved.

The unscaled sum of squares erases tiny nonzero residuals and overflows on
finite large norms. The unchanged source fails three new public-API tests:
tiny and subnormal residuals return success with zero residual, while a
large representable norm returns `NonFiniteMetric`. The fix applies one
native-precision scaled norm algorithm to both metric reductions. Acceptance
uses analytical Pythagorean witnesses, not changed convergence tolerances.

Independent review also reproduces an upper-boundary failure of scaling alone:
a norm below f64::MAX still rounds to infinity. Native compensation through
division, squared terms, reduction, square root and product resolves that
witness; exact-integer f32/f64 certificates distinguish the adjacent genuine
overflow case. Final local, independent review and hosted delivery receipts
belong to the pull request.
This audit does not close broader error-path or transfer coverage items.
