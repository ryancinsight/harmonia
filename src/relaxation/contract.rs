use super::RelaxationError;

/// Fixed-point update policy for a coupled interface pair.
pub trait Relaxation<T> {
    /// Begin a coupling window with no history from an earlier window.
    ///
    /// Stateful policies reset the numerical history used to relate successive
    /// fixed-point iterations. Allocated workspace capacity may be retained.
    /// Stateless policies use this default no-op implementation.
    fn begin_window(&mut self) {}

    /// Update both current interfaces toward their candidates in place.
    ///
    /// The two interfaces are presented together so a stateful policy can
    /// derive one update from the complete coupled defect and retain history
    /// across iterations within the current window. The implementation must
    /// update neither slice when it returns an error.
    ///
    /// # Errors
    ///
    /// Returns a dimension or value failure if the pair cannot be updated.
    fn update_pair(
        &mut self,
        first_current: &mut [T],
        first_candidate: &[T],
        second_current: &mut [T],
        second_candidate: &[T],
    ) -> Result<(), RelaxationError>;
}
