# Harmonia correctness audits

## ATLAS-HARMONIA-WORKSPACE-010 — Workspace/model agreement — 2026-11-05 [patch]

At `f20b831e5508f3b6c82978cf99e18abb0261b39a`, a workspace valid for a
one-output model could be supplied to a different valid two-output model.
Correct caller slices then led the partition export to receive a one-entry
buffer and panic. A matching workspace solves the same linear system.

Each solve now checks all six workspace dimensions and both transfer
destinations against its model before partition callbacks. Each destination is
rechecked immediately before invocation, and returned transfer lengths before
copying. Tests cover both directions, model replacement and recovery, caller
transaction on errors, and analytical matching-workspace controls. The existing
broader error-path and conservation items remain open. Delivery evidence
belongs to the repair pull request; publication remains disabled.

## ATLAS-HARMONIA-AITKEN-RANGE-009 — Aitken secant range — 2026-11-05 [patch]

At `f0ceba6ed8205981f314382da08f059d4bc0743a`, two public-API
regressions fail: opposite finite residuals overflow their difference and
manufacture the minimum factor; a finite FMA update then suffers a spurious
nonfinite secant error from its unscaled product. Open PRs #75/#76 and #88
do not change this path and remain untouched.

Binary exponent separation and native FMA rounding corrections repair both
witnesses. Analytical coverage spans f32/f64 binary scales, subnormal factor
cancellation, the largest finite factor, absolute-tolerance equality, and
transactional rejection of genuine factor overflow. No bound, tolerance,
dependency requirement, external lock record, or publication setting changes.

Source repair and independent source review are complete. The 54 native
tests, MSRV, Clippy, SemVer, supply-chain, and pinned book checks pass.
Final hosted delivery and documentation deployment are tracked by
[Harmonia #91](https://github.com/ryancinsight/harmonia/pull/91). The active
backlog retains only outstanding work; this completed repair is archived here.

## Metric audit — 2026-10-30

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
