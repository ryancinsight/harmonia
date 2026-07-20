use crate::{Partition, Relaxation, Transfer};

/// Associated-type bundle for one coupled partition pair.
///
/// The bundle prevents a chain of independent generic parameters while
/// retaining static dispatch for every constituent policy.
pub trait PairModel<T>
where
    T: Clone,
{
    /// First physics partition.
    type First: Partition<T>;
    /// Second physics partition.
    type Second: Partition<T>;
    /// First-output to second-input transfer.
    type FirstToSecond: Transfer<T>;
    /// Second-output to first-input transfer.
    type SecondToFirst: Transfer<T>;
    /// Shared interface relaxation policy.
    type Relaxation: Relaxation<T>;

    /// Borrow the first partition.
    fn first(&self) -> &Self::First;
    /// Mutably borrow the first partition.
    fn first_mut(&mut self) -> &mut Self::First;
    /// Borrow the second partition.
    fn second(&self) -> &Self::Second;
    /// Mutably borrow the second partition.
    fn second_mut(&mut self) -> &mut Self::Second;
    /// Borrow the first-to-second transfer.
    fn first_to_second(&self) -> &Self::FirstToSecond;
    /// Borrow the second-to-first transfer.
    fn second_to_first(&self) -> &Self::SecondToFirst;
    /// Borrow the relaxation policy.
    fn relaxation(&self) -> &Self::Relaxation;
}

/// Concrete statically dispatched partition-pair bundle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairComponents<First, Second, FirstToSecond, SecondToFirst, Relax> {
    first: First,
    second: Second,
    first_to_second: FirstToSecond,
    second_to_first: SecondToFirst,
    relaxation: Relax,
}

impl<First, Second, FirstToSecond, SecondToFirst, Relax>
    PairComponents<First, Second, FirstToSecond, SecondToFirst, Relax>
{
    /// Bundle two partitions, two directed transfers, and one relaxation
    /// policy.
    #[must_use]
    pub const fn new(
        first: First,
        second: Second,
        first_to_second: FirstToSecond,
        second_to_first: SecondToFirst,
        relaxation: Relax,
    ) -> Self {
        Self {
            first,
            second,
            first_to_second,
            second_to_first,
            relaxation,
        }
    }

    /// Consume the bundle and return its constituents.
    #[must_use]
    pub fn into_parts(self) -> (First, Second, FirstToSecond, SecondToFirst, Relax) {
        (
            self.first,
            self.second,
            self.first_to_second,
            self.second_to_first,
            self.relaxation,
        )
    }
}

impl<T, First, Second, FirstToSecond, SecondToFirst, Relax> PairModel<T>
    for PairComponents<First, Second, FirstToSecond, SecondToFirst, Relax>
where
    T: Clone,
    First: Partition<T>,
    Second: Partition<T>,
    FirstToSecond: Transfer<T>,
    SecondToFirst: Transfer<T>,
    Relax: Relaxation<T>,
{
    type First = First;
    type Second = Second;
    type FirstToSecond = FirstToSecond;
    type SecondToFirst = SecondToFirst;
    type Relaxation = Relax;

    #[inline]
    fn first(&self) -> &Self::First {
        &self.first
    }

    #[inline]
    fn first_mut(&mut self) -> &mut Self::First {
        &mut self.first
    }

    #[inline]
    fn second(&self) -> &Self::Second {
        &self.second
    }

    #[inline]
    fn second_mut(&mut self) -> &mut Self::Second {
        &mut self.second
    }

    #[inline]
    fn first_to_second(&self) -> &Self::FirstToSecond {
        &self.first_to_second
    }

    #[inline]
    fn second_to_first(&self) -> &Self::SecondToFirst {
        &self.second_to_first
    }

    #[inline]
    fn relaxation(&self) -> &Self::Relaxation {
        &self.relaxation
    }
}
