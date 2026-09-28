use super::Substep;

/// Independently advanced physics partition.
///
/// Harmonia validates every slice against the reported dimensions before
/// invoking these methods. Reported dimensions must remain invariant for the
/// lifetime of the partition. Implementations own their numerical method and
/// may reuse internal workspaces, but must not retain any borrowed slice. A
/// checkpoint must contain every internally owned value whose mutation during
/// [`advance`](Self::advance) or [`export`](Self::export) can affect a later
/// advance or export result. Harmonia restores the checkpoint before each
/// fixed-point evaluation so every iteration applies the same map. The
/// checkpoint does not roll the partition back when `solve_window` returns;
/// model state may therefore reflect the last attempted evaluation on error.
pub trait Partition<T> {
    /// Partition-specific failure.
    type Error;

    /// Owned internal state required to replay a coupling window.
    type Checkpoint;

    /// Capture the partition's internal replay state.
    fn checkpoint(&self) -> Self::Checkpoint;

    /// Restore internal state captured by [`checkpoint`](Self::checkpoint).
    fn restore(&mut self, checkpoint: &Self::Checkpoint);

    /// Number of scalar state entries.
    fn state_dimension(&self) -> usize;

    /// Number of incoming interface entries.
    fn input_dimension(&self) -> usize;

    /// Number of exported interface entries.
    fn output_dimension(&self) -> usize;

    /// Advance `state` through one positive typed substep using fixed incoming
    /// interface data.
    ///
    /// # Errors
    ///
    /// Returns the partition's typed numerical failure.
    fn advance(
        &mut self,
        substep: Substep<T>,
        state: &mut [T],
        input: &[T],
    ) -> Result<(), Self::Error>;

    /// Export interface values from the current state into `output`.
    ///
    /// # Errors
    ///
    /// Returns the partition's typed export failure.
    fn export(&self, state: &[T], output: &mut [T]) -> Result<(), Self::Error>;
}
