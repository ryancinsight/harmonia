# 2. Relaxation Policies

Chapter 1 reduced a coupled window to one problem: find \\(x\\) with
\\(F(x) = x\\), where \\(x\\) is the pair of interface inputs and \\(F\\) is one
Jacobi evaluation of both partitions. The obvious iteration is to keep applying
the map,

\\[
x_{k+1} = F(x_k),
\\]

and on many couplings it diverges immediately. This chapter is about the one
knob that fixes it, what the knob can and cannot fix, and why Harmonia refuses
to let that knob touch the convergence test.

## Why the plain iteration diverges

Linearise near the fixed point. With \\(e_k = x_k - x_\star\\) and \\(J\\) the
Jacobian of \\(F\\) at \\(x_\star\\), one step of the plain iteration gives
\\(e_{k+1} = J e_k\\). It converges exactly when the spectral radius of \\(J\\)
is below one, and there is nothing in the physics that arranges that.

The eigenvalues of \\(J\\) are a property of the *coupling*, not of either
partition's solver. \\(J\\) measures how much of a perturbation in the interface
data comes back after a round trip through both fields: perturb the pressure the
structure sees, get a displacement, feed that displacement to the fluid, get a
pressure back. If the returned pressure is larger than the one you injected, the
round trip amplifies and no amount of accuracy in either solver helps.

**Under-relaxation** damps the round trip. Instead of taking the new value,
take a step of length \\(\omega\\) toward it:

\\[
x_{k+1} = x_k + \omega\bigl(F(x_k) - x_k\bigr) = (1-\omega)\,x_k + \omega F(x_k).
\\]

The error now propagates by \\((1-\omega)I + \omega J\\), whose eigenvalues are

\\[
\mu_i = 1 - \omega(1 - \lambda_i),
\\]

with \\(\lambda_i\\) the eigenvalues of \\(J\\). For a real \\(\lambda\\) the
condition \\(|\mu| < 1\\) is

\\[
0 < \omega(1-\lambda) < 2,
\\]

which says three separate things.

If \\(\lambda < 1\\), any \\(\omega\\) in \\((0,\ 2/(1-\lambda))\\) converges.
A strongly overshooting round trip — \\(\lambda\\) large and negative — leaves
only a small window, and \\(\omega \approx 2/|\lambda|\\) is the order of what
survives. This is the case relaxation exists for.

If \\(\lambda > 1\\), no positive \\(\omega\\) satisfies the inequality: the term
\\(\omega(1-\lambda)\\) is negative and \\(|\mu| > 1\\) for every choice. A round
trip that amplifies *in phase* cannot be damped by a convex combination of
\\(x\\) and \\(F(x)\\), because every such combination lies on the segment
between two points that are both on the wrong side. Fixed relaxation is the
wrong tool here and a different one — a quasi-Newton or interface-Newton method,
which builds an approximation to \\(J\\) instead of scaling the step — is
needed.

And for \\(0 < \lambda < 1\\) the admissible range extends past one:
\\(\mu\\) vanishes at \\(\omega = 1/(1-\lambda) > 1\\). Over-relaxation is
optimal for that case. Harmonia's fixed weight is validated into \\((0, 1]\\),
so it cannot express it; the crate's bounded Aitken policy supplies a
componentwise real secant estimate, while Anderson-style acceleration remains
outside this crate. For complex
\\(\lambda\\) the scalar inequality becomes the disc condition — \\(\omega(1 -
\lambda)\\) must lie inside the circle of radius one centred at one — and the
same three regimes appear as its real slice.

## Where the bad eigenvalue comes from: added mass

In fluid–structure interaction the amplifying case is not exotic, and its size
is predictable before any code runs. Accelerating a structure into an
incompressible fluid requires accelerating fluid with it, and to the structure
that appears as extra inertia: an **added-mass** operator acting on the
interface. The round-trip gain scales with the ratio of that added mass to the
structure's own — roughly, the fluid-to-structure density ratio times the
largest eigenvalue of the added-mass operator.

Causin, Gerbeau and Nobile analysed this on a model problem and showed that once
the ratio passes a threshold set by that largest eigenvalue, the explicit,
loosely coupled scheme — one pass of \\(F\\) per window, no iteration — is
*unconditionally* unstable. The word is load-bearing: the instability is not
removed by reducing the time step, so the usual reflex is not merely inefficient
but ineffective. This is the reason a partitioned FSI solver iterates the
interface at all, and the reason it under-relaxes when it does.

The practical shape of the result is that light structures in dense fluids are
hard and heavy structures in light fluids are easy: haemodynamics, with a vessel
wall and blood at comparable densities, sits at the hard end, and a steel wing
in air sits at the easy end. A useful \\(\omega\\) shrinks as the ratio grows.

## Fixed weights and dynamic weights

A single \\(\omega\\) held for the whole solve is the simplest policy and has an
obvious defect: it is chosen for the worst iteration and paid for on all the
others. \\(J\\) is not constant — it varies through the window and across the
simulation as the interface configuration changes — so a weight safe at the
hardest moment is needlessly slow everywhere else, and one tuned to the easy
regime fails at the hard one.

**Dynamic relaxation** re-estimates \\(\omega\\) each iteration from the defect
vectors already computed. The standard method is Aitken's \\(\Delta^2\\)
acceleration in its Irons–Tuck vector form: the last two defects
\\(F(x_k) - x_k\\) and \\(F(x_{k-1}) - x_{k-1}\\) determine a secant estimate of
how fast the defect is contracting, and \\(\omega\\) is updated to the step that
would have annihilated it. Küttler and Wall studied Aitken's \\(\Delta^2\\) and
steepest descent for exactly this purpose and found the choice of relaxation
parameter to be the decisive ingredient in the efficiency of fixed-point FSI
solvers. It costs two vector operations per iteration and no extra evaluation of
\\(F\\).

Harmonia's `Relaxation<T>` seam retains enough state for a policy to implement
dynamic relaxation without changing the coupling loop. `AitkenRelaxation<T>`
owns the previous stacked defect and factor, so it computes the componentwise
Irons--Tuck secant update from the two interface blocks in one call. It costs
two vector operations per iteration and no extra evaluation of `F`. Its
configuration validates positive finite bounds and a positive finite
denominator tolerance in the native precision of `T`.

## What Harmonia provides

Three policies implement `Relaxation<T>`.

| Policy | Weight | Size | Failure |
| --- | --- | --- | --- |
| `AitkenRelaxation<T>` | bounded componentwise secant factors | retained history and reusable workspaces | non-finite input, factor, or update |
| `FullRelaxation` | \\(\omega = 1\\) | zero-sized | non-finite candidate entry |
| `FixedRelaxation<T>` | validated \\(\omega \in (0, 1]\\) | one scalar, `repr(transparent)` | non-finite updated entry |

`FullRelaxation` is the undamped iteration \\(x \leftarrow F(x)\\), written as a
copy with a finiteness check rather than as `FixedRelaxation::new(1.0)`, so the
common case carries no weight to multiply by and no scalar in the pair's layout.
It is the right starting point: run with it, and reach for a weight only when
the defect history says the round trip amplifies.

`FixedRelaxation<T>` validates its weight at construction. A non-finite,
non-positive, or greater-than-one weight is rejected with
`InvalidRelaxation::OutsideUnitInterval`, so a mistyped \\(\omega\\) fails where
it was written rather than producing a solve that cannot converge. The update
itself is

\\[
x_i \leftarrow x_i + \omega\,\bigl(c_i - x_i\bigr),
\\]

evaluated as a fused multiply-add so the multiplication and addition carry one
rounding rather than two, in the precision of `T` itself with no widening.

Both policies check that the current and candidate slices agree in length,
reporting `RelaxationError::Dimension` with both lengths, and both reject a
non-finite result with the offending index. A relaxation that produced a NaN
would otherwise poison the interface silently and reappear several iterations
later as a non-finite metric with no indication of where it started.

One policy serves both interface blocks: the pair model exposes one mutable
`relaxation_mut()`, and the loop passes both guesses to one `update_pair` call.
This matters for stateful policies because a coupled defect and its history are
properties of the stacked interface, not of either block in isolation. The
fixed policy applies the same weight to both blocks, while Aitken computes one
factor per stacked component. A policy error leaves both interfaces and its
retained history unchanged.

The static policies are cost-free at the pair's boundary: the transfer and
relaxation policies are zero-sized wherever they carry no data, and the test
suite asserts it on `IdentityTransfer`, `IndexTransfer<N>`, and
`FullRelaxation`. `AitkenRelaxation<T>` intentionally retains state and reuses
its capacity after the first update; its history allocation is not hidden as a
workspace allocation claim.

## Relaxation cannot manufacture convergence

Here is the trap the crate is built to avoid. A stopping rule written against
the *step* the iteration takes,

\\[
\\|x_{k+1} - x_k\\| = \omega\\,\\|F(x_k) - x_k\\|,
\\]

is proportional to \\(\omega\\). Halve the weight and the measured quantity
halves with it, while the actual defect — the amount by which the two fields
disagree at the interface — has not moved at all. A small enough \\(\omega\\)
makes any coupling look converged on the first iteration. The failure is
particularly ugly because the natural response to a divergent coupling is to
reduce \\(\omega\\), which is precisely the direction that buys a false green.

Harmonia measures \\(\\|F(x_k) - x_k\\|\\), formed before the relaxed update and
never scaled by \\(\omega\\). The relaxed value is computed only after the check
has already failed and only to seed the next iteration; it is not an input to
any convergence decision. \\(\omega\\) therefore influences how many iterations a
solve takes and whether it converges at all, and has no path to influence whether
a given iterate is *called* converged.

The property is tested directly rather than argued: a pair with a constant
non-zero defect is solved with \\(\omega = 10^{-12}\\) and a tolerance set
between the scaled update and the raw defect. The scaled update is far below the
tolerance; the solve must exhaust its budget and report a residual above one, and
it does.

## Choosing a weight in practice

Start at `FullRelaxation`. If the reported defect grows from iteration to
iteration, the round trip amplifies and a weight is needed; if it falls but
slowly, use `AitkenRelaxation<T>` when a bounded secant estimate is appropriate
for the interface contract.

The defect history is the diagnostic, and it is exposed rather than logged.
Athena's `IterationObserver` receives an `IterationState` at every check point
carrying the iteration index, the residual, and the threshold in force; the
observer is a generic parameter, so a caller that wants nothing pays nothing and
a caller that wants the trace records it without Harmonia owning a logging
policy. A diverging solve is visible in that sequence several iterations before
the budget runs out.

Two effects of \\(\omega\\) are worth keeping distinct when reading the trace.
It changes the *rate*: the contraction factor becomes
\\(q_\omega = \max_i |1 - \omega(1-\lambda_i)|\\), and Chapter 1's a-posteriori
bound \\(\\|x - x_\star\\| \le r / (1 - q)\\) uses that \\(q_\omega\\), so a
heavily damped solve converts its defect into a weaker accuracy claim even when
it reaches the same tolerance. And it changes the *cost*: each iteration is a
full advance of both partitions across all their substeps, which is the
expensive thing in the loop, so halving \\(\omega\\) to be safe is not a cheap
insurance policy.

## References

- P. Causin, J.-F. Gerbeau and F. Nobile, "Added-mass effect in the design of
  partitioned algorithms for fluid–structure problems", *Computer Methods in
  Applied Mechanics and Engineering* 194(42–44), 2005, 4506–4527.
  <https://doi.org/10.1016/j.cma.2004.12.005>. The model-problem analysis
  showing that the explicit partitioned scheme is unconditionally unstable once
  the fluid-to-structure density ratio, weighted by the largest eigenvalue of the
  added-mass operator, passes a threshold.
- U. Küttler and W. A. Wall, "Fixed-point fluid–structure interaction solvers
  with dynamic relaxation", *Computational Mechanics* 43(1), 2008, 61–72.
  <https://doi.org/10.1007/s00466-008-0255-5>. Aitken's \\(\Delta^2\\) method and
  steepest descent for computing the relaxation parameter, and their effect on
  the efficiency of fixed-point FSI solvers.
- C. A. Felippa, K. C. Park and C. Farhat, "Partitioned analysis of coupled
  mechanical systems", *Computer Methods in Applied Mechanics and Engineering*
  190(24), 2001, 3247–3270. <https://doi.org/10.1016/S0045-7825(00)00391-1>.
- `docs/adr/0001-partitioned-coupling-boundary.md` — the relaxation-honesty
  theorem and its regression test.
