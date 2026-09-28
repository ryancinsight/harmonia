//! Shared per-interface relaxation machinery.
//!
//! [`FullRelaxation`](super::FullRelaxation) and
//! [`FixedRelaxation`](super::FixedRelaxation) differ only in their pointwise
//! step, so both route through [`update_pair_slices`]. Keeping the length
//! check, the finiteness scan, and the write in one place lets every policy
//! inherit the same transactional "no mutation on error" guarantee.

use eunomia::NumericElement;

use super::RelaxationError;

/// Reject an interface whose current and candidate slices differ in length.
///
/// # Errors
///
/// Returns [`RelaxationError::Dimension`] carrying both lengths.
pub(crate) fn validate_dimensions<T>(
    current: &[T],
    candidate: &[T],
) -> Result<(), RelaxationError> {
    if current.len() == candidate.len() {
        Ok(())
    } else {
        Err(RelaxationError::Dimension {
            current: current.len(),
            candidate: candidate.len(),
        })
    }
}

fn validate_slice<T, F>(
    current: &[T],
    candidate: &[T],
    index_offset: usize,
    step: &F,
) -> Result<(), RelaxationError>
where
    T: NumericElement,
    F: Fn(T, T) -> T,
{
    validate_dimensions(current, candidate)?;
    for (index, (current_value, candidate_value)) in
        current.iter().zip(candidate.iter()).enumerate()
    {
        if !step(*current_value, *candidate_value).is_finite() {
            return Err(RelaxationError::NonFinite {
                index: index_offset + index,
            });
        }
    }
    Ok(())
}

fn apply_slice<T, F>(current: &mut [T], candidate: &[T], step: &F)
where
    T: NumericElement,
    F: Fn(T, T) -> T,
{
    for (current_value, candidate_value) in current.iter_mut().zip(candidate.iter()) {
        *current_value = step(*current_value, *candidate_value);
    }
}

/// Validate and then write a single interface slice through `step`.
///
/// The slice is scanned in full for finiteness before any entry is written, so
/// a rejected update leaves `current` untouched.
///
/// # Errors
///
/// Returns a dimension mismatch or the absolute index of the first entry whose
/// `step` result is non-finite.
pub(crate) fn update_slice<T, F>(
    current: &mut [T],
    candidate: &[T],
    index_offset: usize,
    step: F,
) -> Result<(), RelaxationError>
where
    T: NumericElement,
    F: Fn(T, T) -> T,
{
    validate_slice(current, candidate, index_offset, &step)?;
    apply_slice(current, candidate, &step);
    Ok(())
}

/// Validate and update both interfaces of a coupled pair through `step`.
///
/// The first interface is validated without writing it, then the second is
/// validated and written, then the first is written. Neither interface is
/// mutated until both have passed the finiteness scan, which preserves the
/// transactional contract of [`Relaxation`](super::Relaxation) across the pair
/// while validating each slice before it is written. The scan order matches the
/// concatenated interface so the reported index is offset past the first slice.
///
/// # Errors
///
/// Returns a dimension mismatch on either interface, or the absolute index of
/// the first non-finite update with the second interface offset past the first.
pub(crate) fn update_pair_slices<T, F>(
    first_current: &mut [T],
    first_candidate: &[T],
    second_current: &mut [T],
    second_candidate: &[T],
    step: F,
) -> Result<(), RelaxationError>
where
    T: NumericElement,
    F: Fn(T, T) -> T + Copy,
{
    validate_slice(first_current, first_candidate, 0, &step)?;
    update_slice(second_current, second_candidate, first_current.len(), step)?;
    apply_slice(first_current, first_candidate, &step);
    Ok(())
}
