use core::marker::PhantomData;

use athena_core::{ConvergencePolicy, IterationObserver, IterationState};
use eunomia::{NumericElement, RealField};
use horae::{
    subcycling::{SubcycleError, SubcyclePlan},
    time::{Instant, StepSize, TimeError},
};

use crate::{PairModel, Partition, Relaxation, Substep, Transfer};

use super::{CouplingError, CouplingReport, PairConstructionError, PairWorkspace};

/// Transactional synchronous Jacobi coupling of two partitions.
///
/// The model and scalar parameters are statically dispatched. The two const
/// parameters determine each partition's number of substeps per coupling
/// window.
pub struct PartitionedPair<M, T, const FIRST_SUBSTEPS: usize, const SECOND_SUBSTEPS: usize> {
    model: M,
    workspace: PairWorkspace<T>,
    scalar: PhantomData<T>,
}

impl<M, T, const FIRST_SUBSTEPS: usize, const SECOND_SUBSTEPS: usize>
    PartitionedPair<M, T, FIRST_SUBSTEPS, SECOND_SUBSTEPS>
where
    T: RealField,
    M: PairModel<T>,
{
    /// Construct a statically routed coupling pair.
    ///
    /// # Errors
    ///
    /// Returns a subcycle validation failure for either zero or unrepresentable
    /// const ratio.
    pub fn new(model: M, workspace: PairWorkspace<T>) -> Result<Self, SubcycleError> {
        SubcyclePlan::<FIRST_SUBSTEPS>::new()?;
        SubcyclePlan::<SECOND_SUBSTEPS>::new()?;
        Ok(Self {
            model,
            workspace,
            scalar: PhantomData,
        })
    }

    /// Allocate a validated workspace and construct the pair.
    ///
    /// # Errors
    ///
    /// Returns either a workspace-dimension or subcycle-policy failure.
    pub fn for_model(model: M) -> Result<Self, PairConstructionError> {
        let workspace =
            PairWorkspace::for_model(&model).map_err(PairConstructionError::Workspace)?;
        Self::new(model, workspace).map_err(PairConstructionError::Subcycle)
    }

    /// Borrow the pair model.
    #[must_use]
    pub const fn model(&self) -> &M {
        &self.model
    }

    /// Mutably borrow the pair model.
    #[must_use]
    pub const fn model_mut(&mut self) -> &mut M {
        &mut self.model
    }

    /// Advance one coupling window transactionally.
    ///
    /// The four caller slices are committed together only after the raw
    /// fixed-point defect meets `policy`. Every error leaves all four slices
    /// unchanged.
    ///
    /// # Errors
    ///
    /// Returns exact dimension, time, subcycle, partition, transfer,
    /// relaxation, non-finite, or convergence-budget failures.
    #[expect(
        clippy::too_many_arguments,
        reason = "the transactional boundary keeps four caller-owned slices, typed time, policy, and observer explicit"
    )]
    #[expect(
        clippy::type_complexity,
        reason = "the public error preserves both partition-specific failure types without an architectural alias"
    )]
    pub fn solve_window<O>(
        &mut self,
        start: Instant<T>,
        window: StepSize<T>,
        first_state: &mut [T],
        second_state: &mut [T],
        first_input: &mut [T],
        second_input: &mut [T],
        policy: &ConvergencePolicy<T>,
        observer: &mut O,
    ) -> Result<
        CouplingReport<T>,
        CouplingError<T, <M::First as Partition<T>>::Error, <M::Second as Partition<T>>::Error>,
    >
    where
        O: IterationObserver<T>,
    {
        self.snapshot(first_state, second_state, first_input, second_input)?;

        for iteration in 1..=policy.max_iterations() {
            let (residual_norm, candidate_norm) = self.evaluate(start, window)?;
            let threshold = policy.threshold(candidate_norm);

            if policy.should_check(iteration) {
                observer.observe(IterationState::new(iteration, residual_norm, threshold));
                if residual_norm <= threshold {
                    self.commit(first_state, second_state, first_input, second_input);
                    return Ok(CouplingReport::new(
                        iteration,
                        residual_norm,
                        threshold,
                        FIRST_SUBSTEPS,
                        SECOND_SUBSTEPS,
                    ));
                }
            }

            if iteration == policy.max_iterations() {
                return Err(CouplingError::NotConverged {
                    iterations: iteration,
                    residual_norm,
                    threshold,
                });
            }

            self.relax()?;
        }

        unreachable!("invariant: Athena requires a positive iteration budget")
    }

    #[expect(
        clippy::type_complexity,
        reason = "the internal result preserves both partition failures for the public boundary"
    )]
    fn snapshot(
        &mut self,
        first_state: &[T],
        second_state: &[T],
        first_input: &[T],
        second_input: &[T],
    ) -> Result<
        (),
        CouplingError<T, <M::First as Partition<T>>::Error, <M::Second as Partition<T>>::Error>,
    > {
        for (role, expected, actual) in [
            (
                "first state",
                self.workspace.first_initial.len(),
                first_state.len(),
            ),
            (
                "second state",
                self.workspace.second_initial.len(),
                second_state.len(),
            ),
            (
                "first input",
                self.workspace.first_guess.len(),
                first_input.len(),
            ),
            (
                "second input",
                self.workspace.second_guess.len(),
                second_input.len(),
            ),
        ] {
            validate_dimension(role, expected, actual)?;
        }
        self.workspace.first_initial.copy_from_slice(first_state);
        self.workspace.second_initial.copy_from_slice(second_state);
        self.workspace.first_guess.copy_from_slice(first_input);
        self.workspace.second_guess.copy_from_slice(second_input);
        Ok(())
    }

    #[expect(
        clippy::type_complexity,
        reason = "the internal result preserves both partition failures for the public boundary"
    )]
    fn evaluate(
        &mut self,
        start: Instant<T>,
        window: StepSize<T>,
    ) -> Result<
        (T, T),
        CouplingError<T, <M::First as Partition<T>>::Error, <M::Second as Partition<T>>::Error>,
    > {
        self.workspace
            .first_work
            .copy_from_slice(&self.workspace.first_initial);
        self.workspace
            .second_work
            .copy_from_slice(&self.workspace.second_initial);
        advance_window::<T, _, FIRST_SUBSTEPS>(
            self.model.first_mut(),
            start,
            window,
            &mut self.workspace.first_work,
            &self.workspace.first_guess,
        )
        .map_err(map_first_step)?;
        advance_window::<T, _, SECOND_SUBSTEPS>(
            self.model.second_mut(),
            start,
            window,
            &mut self.workspace.second_work,
            &self.workspace.second_guess,
        )
        .map_err(map_second_step)?;
        self.export_and_transfer()?;
        pair_metrics(
            &self.workspace.first_guess,
            &self.workspace.first_candidate,
            &self.workspace.second_guess,
            &self.workspace.second_candidate,
        )
        .ok_or(CouplingError::NonFiniteMetric)
    }

    #[expect(
        clippy::type_complexity,
        reason = "the internal result preserves both partition failures for the public boundary"
    )]
    fn export_and_transfer(
        &mut self,
    ) -> Result<
        (),
        CouplingError<T, <M::First as Partition<T>>::Error, <M::Second as Partition<T>>::Error>,
    > {
        self.model
            .first()
            .export(&self.workspace.first_work, &mut self.workspace.first_output)
            .map_err(CouplingError::First)?;
        self.model
            .second()
            .export(
                &self.workspace.second_work,
                &mut self.workspace.second_output,
            )
            .map_err(CouplingError::Second)?;
        let second_candidate = self
            .model
            .first_to_second()
            .transfer(
                &self.workspace.first_output,
                &mut self.workspace.second_transfer,
            )
            .map_err(CouplingError::Transfer)?;
        self.workspace
            .second_candidate
            .copy_from_slice(second_candidate.as_ref());
        let first_candidate = self
            .model
            .second_to_first()
            .transfer(
                &self.workspace.second_output,
                &mut self.workspace.first_transfer,
            )
            .map_err(CouplingError::Transfer)?;
        self.workspace
            .first_candidate
            .copy_from_slice(first_candidate.as_ref());
        Ok(())
    }

    #[expect(
        clippy::type_complexity,
        reason = "the internal result preserves both partition failures for the public boundary"
    )]
    fn relax(
        &mut self,
    ) -> Result<
        (),
        CouplingError<T, <M::First as Partition<T>>::Error, <M::Second as Partition<T>>::Error>,
    > {
        self.model
            .relaxation()
            .update(
                &mut self.workspace.first_guess,
                &self.workspace.first_candidate,
            )
            .map_err(CouplingError::Relaxation)?;
        self.model
            .relaxation()
            .update(
                &mut self.workspace.second_guess,
                &self.workspace.second_candidate,
            )
            .map_err(CouplingError::Relaxation)
    }

    fn commit(
        &self,
        first_state: &mut [T],
        second_state: &mut [T],
        first_input: &mut [T],
        second_input: &mut [T],
    ) {
        first_state.copy_from_slice(&self.workspace.first_work);
        second_state.copy_from_slice(&self.workspace.second_work);
        first_input.copy_from_slice(&self.workspace.first_candidate);
        second_input.copy_from_slice(&self.workspace.second_candidate);
    }

    /// Consume the coupling and return its model and reusable workspace.
    #[must_use]
    pub fn into_parts(self) -> (M, PairWorkspace<T>) {
        (self.model, self.workspace)
    }
}

enum WindowStepError<E> {
    Time(TimeError),
    Subcycle(SubcycleError),
    Partition(E),
}

fn advance_window<T, P, const SUBSTEPS: usize>(
    partition: &mut P,
    start: Instant<T>,
    window: StepSize<T>,
    state: &mut [T],
    input: &[T],
) -> Result<(), WindowStepError<P::Error>>
where
    T: RealField,
    P: Partition<T>,
{
    let plan = SubcyclePlan::<SUBSTEPS>::new().map_err(WindowStepError::Subcycle)?;
    let child = plan.child_step(window).map_err(WindowStepError::Subcycle)?;
    let endpoint = start.advance(window).map_err(WindowStepError::Time)?;
    let mut cursor = start;

    for index in 0..SUBSTEPS {
        let size = if index + 1 == SUBSTEPS {
            endpoint
                .duration_since(cursor)
                .map_err(WindowStepError::Time)?
        } else {
            child
        };
        partition
            .advance(Substep::new(cursor, size, index, SUBSTEPS), state, input)
            .map_err(WindowStepError::Partition)?;
        cursor = cursor.advance(size).map_err(WindowStepError::Time)?;
    }
    Ok(())
}

fn pair_metrics<T>(
    first_current: &[T],
    first_candidate: &[T],
    second_current: &[T],
    second_candidate: &[T],
) -> Option<(T, T)>
where
    T: RealField,
{
    let mut residual_squared = <T as NumericElement>::ZERO;
    let mut candidate_squared = <T as NumericElement>::ZERO;
    for (current, candidate) in first_current
        .iter()
        .chain(second_current)
        .copied()
        .zip(first_candidate.iter().chain(second_candidate).copied())
    {
        let difference = candidate - current;
        residual_squared = difference.scalar_fmadd(difference, residual_squared);
        candidate_squared = candidate.scalar_fmadd(candidate, candidate_squared);
    }
    let residual = residual_squared.sqrt();
    let candidate = candidate_squared.sqrt();
    (residual.is_finite() && candidate.is_finite()).then_some((residual, candidate))
}

fn validate_dimension<T, FirstError, SecondError>(
    role: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), CouplingError<T, FirstError, SecondError>> {
    if expected == actual {
        Ok(())
    } else {
        Err(CouplingError::Dimension {
            role,
            expected,
            actual,
        })
    }
}

fn map_first_step<T, FirstError, SecondError>(
    error: WindowStepError<FirstError>,
) -> CouplingError<T, FirstError, SecondError> {
    match error {
        WindowStepError::Time(error) => CouplingError::Time(error),
        WindowStepError::Subcycle(error) => CouplingError::Subcycle(error),
        WindowStepError::Partition(error) => CouplingError::First(error),
    }
}

fn map_second_step<T, FirstError, SecondError>(
    error: WindowStepError<SecondError>,
) -> CouplingError<T, FirstError, SecondError> {
    match error {
        WindowStepError::Time(error) => CouplingError::Time(error),
        WindowStepError::Subcycle(error) => CouplingError::Subcycle(error),
        WindowStepError::Partition(error) => CouplingError::Second(error),
    }
}
