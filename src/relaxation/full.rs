use eunomia::NumericElement;

use super::{Relaxation, RelaxationError, slice::update_pair_slices};

/// Zero-sized full fixed-point update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FullRelaxation;

impl<T> Relaxation<T> for FullRelaxation
where
    T: NumericElement,
{
    fn update_pair(
        &mut self,
        first_current: &mut [T],
        first_candidate: &[T],
        second_current: &mut [T],
        second_candidate: &[T],
    ) -> Result<(), RelaxationError> {
        // Full relaxation is a pure copy: the pointwise step ignores the current
        // value and returns the candidate bit-for-bit. It deliberately is not
        // `scalar_fmadd(1, candidate - current, current)`, which would round
        // differently; `tests/codegen_equivalence.rs` pins this copy exactly.
        update_pair_slices(
            first_current,
            first_candidate,
            second_current,
            second_candidate,
            |_, candidate| candidate,
        )
    }
}
