use horae::time::{Instant, StepSize};

/// One typed child interval in a coupling time window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Substep<T> {
    start: Instant<T>,
    size: StepSize<T>,
    index: usize,
    count: usize,
}

impl<T> Substep<T> {
    pub(crate) const fn new(
        start: Instant<T>,
        size: StepSize<T>,
        index: usize,
        count: usize,
    ) -> Self {
        Self {
            start,
            size,
            index,
            count,
        }
    }

    /// Start instant.
    #[inline]
    #[must_use]
    pub const fn start(&self) -> &Instant<T> {
        &self.start
    }

    /// Positive step size.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> &StepSize<T> {
        &self.size
    }

    /// Zero-based substep index.
    #[inline]
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Total substeps in this partition's window.
    #[inline]
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
}
