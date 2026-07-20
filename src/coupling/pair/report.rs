/// Successful coupling-window outcome.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CouplingReport<T> {
    /// Completed fixed-point iterations.
    pub iterations: usize,
    /// Final unrelaxed Euclidean fixed-point defect.
    pub residual_norm: T,
    /// Effective Athena convergence threshold.
    pub threshold: T,
    /// First-partition substeps per fixed-point iteration.
    pub first_substeps: usize,
    /// Second-partition substeps per fixed-point iteration.
    pub second_substeps: usize,
}

impl<T> CouplingReport<T> {
    pub(crate) const fn new(
        iterations: usize,
        residual_norm: T,
        threshold: T,
        first_substeps: usize,
        second_substeps: usize,
    ) -> Self {
        Self {
            iterations,
            residual_norm,
            threshold,
            first_substeps,
            second_substeps,
        }
    }
}
