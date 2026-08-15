# 1. Partition and Substep

A coupled multiphysics problem is one where two fields cannot be solved
separately because each supplies the other's boundary data: a fluid loads a
structure while the structure's motion moves the fluid domain; a solid conducts
heat into a flow whose convection sets the solid's surface temperature. Written
out in full, such a problem is a single system in all the unknowns of both
fields, and solving that system directly — *monolithically* — is the
mathematically obvious thing to do.

Almost nobody does it. This chapter is about the alternative and about the one
object that the alternative turns out to be.

## Why a coupled problem gets solved by alternating single-physics solves

The monolithic route asks for a discretisation, a Jacobian, and a linear solver
for the union of two fields. Each field already has all three, tuned over years
to its own equations: a pressure Poisson solve wants a multigrid-preconditioned
Krylov method on a symmetric positive definite operator, a structural dynamics
solve wants something else entirely, and the coupled Jacobian is neither. Worse,
the two are usually different codes.

The **partitioned** approach keeps them separate. Each field is advanced by its
own solver over a shared interval, the fields exchange interface data, and the
exchange is repeated until the interface values stop changing. The single-physics
solvers are used unmodified; only the interface traffic is new
(Felippa, Park and Farhat 2001). What the partitioned approach costs is that the
exchange has to converge, and whether it does is a question about the coupling
and not about either physics — which is why it deserves a crate of its own.

Harmonia is that crate. It owns the exchange and nothing else: it never sees an
equation, a mesh, or a material law.

## The one object: an interface fixed-point map

Fix a **coupling window** \\([t, t + \Delta t]\\) and call the two fields the
first and second partition. Each partition has an internal state
\\(y_i\\) and consumes an interface input \\(u_i\\); after advancing it exports
an interface output \\(g_i\\), and a directed transfer maps one field's output
onto the other's input.

Write \\(x = (u_1, u_2)\\) for the pair of interface inputs at the start of the
window. One *evaluation* does this:

\\[
\begin{aligned}
y_1 &\leftarrow A_1(y_1^0,\ u_1), \qquad g_1 = E_1(y_1),\\\\
y_2 &\leftarrow A_2(y_2^0,\ u_2), \qquad g_2 = E_2(y_2),
\end{aligned}
\\]

and then delivers each output to the other side,

\\[
F(x) = \bigl(\ T_{2\to 1}(g_2),\ \ T_{1\to 2}(g_1)\ \bigr).
\\]

Both advances start from \\(y^0\\), the state snapshot taken at the window start,
and both read the *same* \\(x\\). That is the defining property of a **Jacobi**
iteration: the two partitions are evaluated independently and neither sees the
other's update within the sweep. The alternative — feed \\(u_2\\) the freshly
computed \\(g_1\\) before advancing the second partition — is Gauss-Seidel, and
Harmonia's Phase 0 does not implement it.

Everything Harmonia does is now one sentence: **find \\(x\\) with \\(F(x) = x\\).**
A coupled solution over the window is exactly an interface vector that
reproduces itself, because at such a vector each partition's boundary data is
what the other partition actually produced. Chapter 2 is about making the
iteration that finds it converge; Chapter 3 is about who owns which piece of
\\(F\\).

Restoring \\(y^0\\) before every evaluation is what makes \\(F\\) a map at all.
Harmonia copies the snapshot back into the working state at the top of each
iteration, so an evaluation depends on \\(x\\) alone. A `Partition`
implementation that carries state between calls — an adaptive step controller
holding its last accepted step, an accumulator that is not reset — breaks that:
the second evaluation at the same \\(x\\) returns something else, \\(F\\) is not
a function, and the quantity the loop measures no longer means distance from a
fixed point. Internal scratch buffers are fine; internal *memory* is not.

## Substeps: the window is not a time step

The coupling window is the interval over which the two fields exchange data. It
is not the time step either field integrates with. A partition may need many
steps to cross one window — an explicit scheme bounded by a CFL condition, a
stiff reaction term, an accuracy requirement the other field does not share —
and the number it needs generally differs between the two partitions. Advancing
each field with its own step count inside a shared window is **subcycling**, and
it is the practical reason the window exists as a separate concept: exchanging
at every fine step of the stiffest field would be needless traffic, and forcing
both fields onto the finer field's step would be needless work.

Harmonia fixes each partition's count in the type:

```text
PartitionedPair<M, T, FIRST_SUBSTEPS, SECOND_SUBSTEPS>
```

`FIRST_SUBSTEPS` and `SECOND_SUBSTEPS` are const generic parameters, so the two
counts are independent and each solve monomorphises to its own substep loop with
no runtime branch. Validation is Horae's: `SubcyclePlan<N>` rejects `N = 0` and
any `N` beyond `u32::MAX`, and derives the child step as \\(\Delta t\\) scaled by
the representable reciprocal of \\(N\\).

That reciprocal is generally not exact, so \\(N\\) applications of the child step
do not sum bit-exactly to \\(\Delta t\\). Harmonia does not let the drift
accumulate into the window: the first \\(N-1\\) substeps use Horae's child step
and the last one is the typed duration from the running cursor to the window
endpoint. The final advance therefore lands on the endpoint exactly, whatever the
rounding did in between.

Each call receives a `Substep<T>` carrying the start instant, the positive step
size, the zero-based index, and the total count — enough for a partition to know
where in the window it is without being told separately.

## What the interface does during a window

The interface input handed to `advance` is the *same slice* for every substep of
the window. The partitions do not exchange inside a window; they exchange
between windows. Harmonia's interface waveform is a zeroth-order hold.

Two consequences follow, and they are worth separating.

Subcycling refines each partition's own integration. It does not refine the
coupling. Adding substeps buys accuracy in \\(A_i\\) and buys nothing
in the interface data \\(A_i\\) is given. Harmonia's differential test
makes the null version of this concrete: on a pair whose exact result is
invariant under subdivision, the `<1, 1>` and `<2, 3>` specialisations agree to
a few units in the last place.

And a converged window is the exact solution of a problem in which the interface
was constant across the window — not of the continuously coupled problem. The
gap is the splitting error of the hold, and the only lever on it is the window
size. Removing it needs an interface *waveform* that varies within the window,
which is the subject of waveform relaxation (Meisrimel and Birken 2021) and is
outside Phase 0.

## The iteration

```text
snapshot  y1_0, y2_0, x  <- caller slices          (validated against workspace)
for k in 1 ..= max_iterations:
    y1, y2 <- y1_0, y2_0                            restore the snapshot
    advance y1 over FIRST_SUBSTEPS  substeps with u1
    advance y2 over SECOND_SUBSTEPS substeps with u2
    export g1, g2; transfer to obtain F(x)
    r     <- ||F(x) - x||                           raw defect, before relaxation
    tau   <- max(abs_tol, rel_tol * ||F(x)||)
    if k is a check point:
        observe (k, r, tau)
        if r <= tau:
            commit y1, y2, F(x) into the caller slices
            return report
    if k == max_iterations:
        return NotConverged { k, r, tau }
    x <- x + omega (F(x) - x)                       relaxation, Chapter 2
```

Three details in that loop are decisions rather than mechanics.

**The defect is measured before relaxation.** \\(r = \\|F(x) - x\\|_2\\) is
formed from the raw evaluation; the relaxed update never reaches the convergence
test. Chapter 2 explains why that is the difference between a stopping rule and
a lie.

**The relative scale is \\(\\|F(x)\\|_2\\).** Athena's `ConvergencePolicy`
combines an absolute and a relative tolerance as
\\(\tau = \max(\tau_{\text{abs}},\ \tau_{\text{rel}} \\|\cdot\\|)\\), and the norm
Harmonia supplies is the norm of the candidate interface vector. The test is
therefore scale-free in the interface magnitude, and the absolute term is what
keeps it reachable when the interface values are near zero. Both norms are taken
over the two interface blocks stacked into one vector, so the two fields share a
single stopping criterion rather than each declaring victory separately.

**The check interval gates acceptance, not evaluation.** `check_interval` on the
policy exists for solvers where recomputing a residual costs an extra operator
application. Harmonia has no such saving: it needs \\(F(x)\\) to relax, so it
forms the defect every iteration regardless. Setting an interval above one only
suppresses observation and defers acceptance.

## What a converged window guarantees

The defect is what you can measure; the distance to the fixed point is what you
want. For a contraction with factor \\(q < 1\\) they are related by the standard
a-posteriori estimate

\\[
\\|x - x_\star\\| \le \frac{\\|F(x) - x\\|}{1 - q},
\\]

proved in `docs/adr/0001-partitioned-coupling-boundary.md`. The factor
\\(1/(1-q)\\) is the whole content of the bound: a defect of \\(10^{-10}\\) on a
coupling with \\(q = 0.99\\) certifies \\(10^{-8}\\), not \\(10^{-10}\\), and
\\(q\\) is a property of the coupled physics that Harmonia cannot observe. The
crate reports \\(r\\); converting it into a statement about your interface is
this inequality, and it needs an estimate of \\(q\\) that has to come from you.

The bound is checked, not just asserted. Harmonia's test suite solves a linear
coupled pair analytically, computes \\(q\\) in closed form, and asserts the
inequality against the reported defect — once on a fixed instance and once over
generated contractive pairs, with a rounding allowance derived from machine
epsilon.

## The transaction

A window solve either succeeds and writes all four caller slices, or fails and
writes none of them. Every evaluation happens in workspace buffers; the only
writes to caller memory are the four `copy_from_slice` calls in the convergence
branch, and every error path returns before reaching it. Budget exhaustion is an
error like any other, so a non-converged window leaves the caller's state exactly
as it was and the caller can retry with a smaller window rather than having to
detect and undo a partial advance. The regression test forces non-convergence and
compares all four slices bit-for-bit.

What gets committed on success is worth being precise about: the states are the
work states produced by the accepted evaluation, and the interface values are
\\(F(x)\\), not the relaxed iterate \\(x\\). Those differ by the defect, which is
at most \\(\tau\\) — so the committed interface is the one the partitions actually
produced, consistent with the committed states to within the tolerance you asked
for.

## What a partition must promise

The `Partition<T>` trait is the whole contract between Harmonia and your physics.

| Method | Obligation |
| --- | --- |
| `state_dimension`, `input_dimension`, `output_dimension` | invariant for the lifetime of the partition; the workspace is sized from them once |
| `advance` | advance `state` across one positive substep using `input` held fixed |
| `export` | write the interface values implied by `state` |
| `Error` | the partition's own typed failure, surfaced verbatim |

Three obligations do not appear in the signatures.

Dimensions must not change after construction, because `PairWorkspace::for_model`
allocates against them once and every later slice is validated against those
lengths.

No borrowed slice may be retained. Harmonia hands out `&mut [T]` into its own
buffers and reuses them across iterations.

And, as above, `advance` must be replayable: same snapshot state, same input,
same substep — same result.

Dimension agreement across the pair is checked rather than assumed. Workspace
construction rejects a zero-sized state or interface, and it rejects a transfer
whose destination dimension does not match the receiving partition's input, so a
mismatched pair fails at construction with the offending role named rather than
part-way through a solve.

See [Example: Coupled Decay](examples/coupled_decay.md) for the smallest complete
program: two decaying scalars, heterogeneous substep counts, one window.

## References

- C. A. Felippa, K. C. Park and C. Farhat, "Partitioned analysis of coupled
  mechanical systems", *Computer Methods in Applied Mechanics and Engineering*
  190(24), 2001, 3247–3270. <https://doi.org/10.1016/S0045-7825(00)00391-1>.
  The tutorial statement of partitioned analysis: spatial decomposition into
  partitions advanced separately in time, with interaction carried by
  transmission and synchronisation of coupled state variables.
- P. Meisrimel and P. Birken, "Waveform Relaxation with asynchronous
  time-integration", 2021. <https://arxiv.org/abs/2106.13147>. Waveform
  relaxation for partitioned time integration of surface-coupled problems,
  including the classical Jacobi and Gauss-Seidel variants and the independent
  time grids they admit.
- `docs/adr/0001-partitioned-coupling-boundary.md` — the Phase 0 algorithm, the
  transaction and contraction theorems with their proofs, and the alternatives
  rejected.
