# ADR 0003: Provider-owned Aitken relaxation policy

- Status: Accepted
- Change class: minor, architectural
- Date: 2026-08-18
- Board item: `ATLAS-HARMONIA-AITKEN-001`

## Context

The pair-level `Relaxation<T>` contract now supplies both interface blocks to
one mutable policy. CFDrs has a componentwise Aitken policy that retains the
previous stacked residual and relaxation factor, but its consumer-owned
implementation resets to a target or a floor when an intermediate value is
invalid. That masks a numerical failure and prevents direct provider
integration.

## Decision

Harmonia owns `AitkenRelaxation<T>`, a generic `Relaxation<T>` implementation
over Eunomia's native `RealField` scalar. It concatenates the first and second
interfaces for history, starts from unit factors clamped to the configured
interval on the first update, and computes
each later factor as

\[
\omega_{i,k} = -\omega_{i,k-1}\frac{r_{i,k-1}}{r_{i,k}-r_{i,k-1}},
\qquad r_k = F(x_k)-x_k.
\]

When the denominator is within the configured native-precision tolerance, the
previous factor is retained. Otherwise the computed factor is clamped to the
validated positive `[minimum, maximum]` interval. The update is
`x[i] <- x[i] + omega[i] * r[i]`, evaluated through Eunomia's scalar fused
multiply-add.

Configuration validation rejects non-finite bounds or tolerance, a
non-positive minimum or tolerance, and an upper bound below the lower bound.
An update validates both dimensions, all inputs, all residuals, and all
outputs before changing either interface or the retained history. A
non-finite secant factor is reported as `RelaxationError::NonFinite`; it is
never replaced with a target, zero, or floor value. A change in the pair's
total dimension starts a fresh history on the next successful update.

The policy retains reusable vector capacity after its first update. Static
`FullRelaxation` and `FixedRelaxation` remain allocation-free; the stateful
policy's history allocation is explicit in its ownership and documentation.

## Rejected alternatives

- Retain the CFDrs wrapper: rejected because the consumer would continue to
  own a provider-role policy and its invalid-value reset would mask failures.
- Add a fixed-relaxation fallback for invalid secant values: rejected because
  it changes a numerical failure into silent degradation.
- Copy Anderson acceleration into Harmonia: rejected because Leto already
  owns that algorithm and one source of truth is required.
- Add an adapter around the old wrapper: rejected because the direct provider
  policy is the migration boundary; compatibility layers would preserve the
  superseded ownership.

## Verification

- Analytical componentwise secant and clamp oracles over both pair interfaces.
- Small-denominator history-reuse regression.
- Transactional dimension and non-finite failure tests, including retained
  history after a rejected update.
- Generic `f32` and `f64` instantiations with native-precision arithmetic.
- Warning-denied locked check, Nextest, doctest, Rustdoc, and book build at the
  exact provider head.
