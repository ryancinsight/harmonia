# harmonia -- Multiphysics Coupling for Atlas

`harmonia` couples two physics solvers that each already work on their own.

This book is written for someone meeting partitioned coupling for the first
time. It builds the mathematics before the API: Part I derives what a
partitioned coupling iteration is, why it needs damping to converge, and what a
converged window does and does not guarantee; Part II places the crate in the
Atlas stack and says what it delegates.

## The problem

Two fields are **coupled** when neither can be solved without the other's
answer. A structure deflects under fluid pressure and the deflection changes the
pressure; a solid conducts heat into a flow whose convection sets the solid's
surface temperature. Treating the pair as one system in all the unknowns of both
fields — solving it *monolithically* — is correct and is rarely what happens,
because each field already has a discretisation, a Jacobian, and a linear solver
built for its own equations, and the combined system has none of those.

The **partitioned** approach keeps the two solvers separate and makes the
interface the only shared object. Over a shared interval each field is advanced
by its own solver using the other's interface data held fixed; the fields then
exchange what they produced, and the exchange repeats until the interface values
stop changing.

That last clause is where the difficulty moved to. The exchange is an iteration,
and it can fail to converge for reasons that live in neither physics — a light
structure in a dense fluid diverges no matter how good the two solvers are. What
the partitioned approach buys in solver reuse, it pays for in a convergence
problem of its own, and that problem is what this crate is about.

## One object

Everything here reduces to a single map. Write \\(x\\) for the pair of interface
values the two partitions consume, and \\(F(x)\\) for what comes back after
advancing both fields over one window and exchanging their exports. A coupled
solution over that window is exactly a vector that reproduces itself:

\\[
F(x) = x .
\\]

Chapter 1 constructs \\(F\\) and says what a partition must promise for it to be
a well-defined map at all, what a coupling window is as distinct from a time
step, and what the defect \\(\\|F(x) - x\\|\\) certifies about the answer.
Chapter 2 is about the iteration that finds the fixed point, why the plain
iteration \\(x \leftarrow F(x)\\) diverges on physically ordinary problems, and
what a relaxation weight can and cannot repair. Chapter 3 draws the ownership
boundary around \\(F\\): Harmonia owns the loop, the exchange, and the
transaction, and delegates time, convergence, scalars, and all the physics.

## What the code looks like

Four pieces compose a coupling: two **partitions** supplying the advance and the
interface export, two directed **transfers** carrying each output to the other's
input, a **relaxation** policy damping the update, and a **workspace** holding
the buffers so that repeated windows allocate nothing. Athena's convergence
policy decides when to stop and Horae's subcycle plans decide how many steps each
partition takes inside a window.

See [Example: Coupled Decay](examples/coupled_decay.md) for the smallest
complete program.

## Scope

Phase 0 is one coupling family: two partitions, synchronous Jacobi iteration,
one time window, a zeroth-order-hold interface. Waveform interpolation, partition
graphs, Gauss-Seidel ordering, quasi-Newton acceleration, distributed scheduling,
and conservation-aware nonmatching-mesh transfer are excluded, and are excluded
rather than stubbed — there is no feature flag and no unimplemented trait method
standing in for any of them. [Position in the Stack](stack_position.md) lists
each exclusion with the contract it is waiting on.
