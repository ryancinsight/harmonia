use super::RelaxationError;

/// Fixed-point interface update policy.
pub trait Relaxation<T> {
    /// Update `current` toward `candidate` in place.
    ///
    /// # Errors
    ///
    /// Returns a value failure if an updated entry is non-finite.
    fn update(&self, current: &mut [T], candidate: &[T]) -> Result<(), RelaxationError>;
}
