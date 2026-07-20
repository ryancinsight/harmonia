use alloc::{boxed::Box, vec};
use eunomia::NumericElement;

use crate::{PairModel, Partition, Transfer};

use super::WorkspaceError;

/// Reusable fixed-size storage for one pair model.
///
/// All allocation occurs in [`for_model`](Self::for_model). Window solves
/// mutate these boxes in place.
pub struct PairWorkspace<T> {
    pub(super) first_initial: Box<[T]>,
    pub(super) first_work: Box<[T]>,
    pub(super) second_initial: Box<[T]>,
    pub(super) second_work: Box<[T]>,
    pub(super) first_output: Box<[T]>,
    pub(super) second_output: Box<[T]>,
    pub(super) first_guess: Box<[T]>,
    pub(super) second_guess: Box<[T]>,
    pub(super) first_candidate: Box<[T]>,
    pub(super) second_candidate: Box<[T]>,
    pub(super) first_transfer: Box<[T]>,
    pub(super) second_transfer: Box<[T]>,
}

impl<T> PairWorkspace<T>
where
    T: NumericElement,
{
    /// Allocate and validate storage for `model`.
    ///
    /// # Errors
    ///
    /// Returns the exact zero or transfer-dimension mismatch.
    pub fn for_model<M>(model: &M) -> Result<Self, WorkspaceError>
    where
        M: PairModel<T>,
    {
        let first_state = model.first().state_dimension();
        let second_state = model.second().state_dimension();
        let first_input = model.first().input_dimension();
        let second_input = model.second().input_dimension();
        let first_output = model.first().output_dimension();
        let second_output = model.second().output_dimension();

        for (role, dimension) in [
            ("first state", first_state),
            ("second state", second_state),
            ("first input", first_input),
            ("second input", second_input),
            ("first output", first_output),
            ("second output", second_output),
        ] {
            if dimension == 0 {
                return Err(WorkspaceError::ZeroDimension { role });
            }
        }

        let second_transfer = model.first_to_second().destination_dimension(first_output);
        if second_transfer != second_input {
            return Err(WorkspaceError::TransferDimension {
                role: "first-to-second transfer",
                transfer: second_transfer,
                input: second_input,
            });
        }
        let first_transfer = model.second_to_first().destination_dimension(second_output);
        if first_transfer != first_input {
            return Err(WorkspaceError::TransferDimension {
                role: "second-to-first transfer",
                transfer: first_transfer,
                input: first_input,
            });
        }

        Ok(Self {
            first_initial: zeros(first_state),
            first_work: zeros(first_state),
            second_initial: zeros(second_state),
            second_work: zeros(second_state),
            first_output: zeros(first_output),
            second_output: zeros(second_output),
            first_guess: zeros(first_input),
            second_guess: zeros(second_input),
            first_candidate: zeros(first_input),
            second_candidate: zeros(second_input),
            first_transfer: zeros(first_input),
            second_transfer: zeros(second_input),
        })
    }
}

fn zeros<T>(dimension: usize) -> Box<[T]>
where
    T: NumericElement,
{
    vec![<T as NumericElement>::ZERO; dimension].into_boxed_slice()
}
