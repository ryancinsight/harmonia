//! Transaction theorem on every reachable pair-coupling error path.
//!
//! ADR 0001 states that every `solve_window` failure leaves all four caller
//! slices unchanged. Each test below drives one `CouplingError` variant to its
//! public return and compares the caller slices bit for bit with their entry
//! values. `NotConverged` and `NonFiniteMetric` are covered by the sibling
//! `nonconvergence_is_transactional` test and `tests/metrics.rs`.

use aequitas::systems::si::quantities::Time;
use athena_core::ConvergencePolicy;
use harmonia::{
    CouplingError, FullRelaxation, IdentityTransfer, IndexTransfer, PairComponents, PairModel,
    Partition, PartitionedPair, Relaxation, RelaxationError, Substep, TransferError,
};
use horae::{
    subcycling::SubcycleError,
    time::{Instant, StepSize, TimeError},
};

use super::support::{ConstantOutput, LastObserver, instant, window};

type FirstError<M> = <<M as PairModel<f64>>::First as Partition<f64>>::Error;
type SecondError<M> = <<M as PairModel<f64>>::Second as Partition<f64>>::Error;
type Rejection<M> = CouplingError<f64, FirstError<M>, SecondError<M>>;

const FIRST_STATE: usize = 0;
const SECOND_STATE: usize = 1;
const FIRST_INPUT: usize = 2;
const SECOND_INPUT: usize = 3;

/// Four caller-owned slices in `solve_window` argument order.
struct Caller([Vec<f64>; 4]);

impl Caller {
    fn nominal() -> Self {
        Self([vec![11.0], vec![12.0], vec![13.0], vec![14.0]])
    }

    fn bits(&self) -> [Vec<u64>; 4] {
        [&self.0[0], &self.0[1], &self.0[2], &self.0[3]]
            .map(|slice| slice.iter().map(|value| value.to_bits()).collect())
    }

    /// Solve one window from `start` and require a rejection that preserves
    /// every caller bit.
    fn rejection<M, const FIRST: usize, const SECOND: usize>(
        &mut self,
        pair: &mut PartitionedPair<M, f64, FIRST, SECOND>,
        start: Instant<f64>,
        span: StepSize<f64>,
        max_iterations: usize,
    ) -> Rejection<M>
    where
        M: PairModel<f64>,
    {
        let policy = ConvergencePolicy::new(0.0, 0.0, max_iterations)
            .expect("invariant: positive iteration budget");
        let before = self.bits();
        let [first_state, second_state, first_input, second_input] = &mut self.0;
        let result = pair.solve_window(
            start,
            span,
            first_state,
            second_state,
            first_input,
            second_input,
            &policy,
            &mut LastObserver::default(),
        );
        let Err(error) = result else {
            panic!("the window must be rejected");
        };
        assert_eq!(self.bits(), before, "a rejected window changed caller bits");
        error
    }
}

/// Partition that mutates its work state, then fails on a chosen substep or
/// at export.
#[derive(Clone, Copy, Debug)]
struct Faulty {
    advance_failure: Option<usize>,
    export_failure: bool,
}

impl Faulty {
    const HEALTHY: Self = Self {
        advance_failure: None,
        export_failure: false,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    Advance { substep: usize },
    Export,
}

impl Partition<f64> for Faulty {
    type Error = Fault;
    type Checkpoint = ();

    fn checkpoint(&self) -> Self::Checkpoint {}

    fn restore(&mut self, _checkpoint: &Self::Checkpoint) {}

    fn state_dimension(&self) -> usize {
        1
    }

    fn input_dimension(&self) -> usize {
        1
    }

    fn output_dimension(&self) -> usize {
        1
    }

    fn advance(
        &mut self,
        substep: Substep<f64>,
        state: &mut [f64],
        _input: &[f64],
    ) -> Result<(), Self::Error> {
        state[0] += 1.0;
        if self.advance_failure == Some(substep.index()) {
            return Err(Fault::Advance {
                substep: substep.index(),
            });
        }
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        if self.export_failure {
            return Err(Fault::Export);
        }
        output.copy_from_slice(state);
        Ok(())
    }
}

/// Relaxation policy whose every update is rejected.
struct RejectingRelaxation;

impl Relaxation<f64> for RejectingRelaxation {
    fn update_pair(
        &mut self,
        _first_current: &mut [f64],
        _first_candidate: &[f64],
        _second_current: &mut [f64],
        _second_candidate: &[f64],
    ) -> Result<(), RelaxationError> {
        Err(RelaxationError::NonFinite { index: 1 })
    }
}

type FaultyModel =
    PairComponents<Faulty, Faulty, IdentityTransfer, IdentityTransfer, FullRelaxation>;

fn faulty_pair<const FIRST: usize, const SECOND: usize>(
    first: Faulty,
    second: Faulty,
) -> PartitionedPair<FaultyModel, f64, FIRST, SECOND> {
    PartitionedPair::for_model(PairComponents::new(
        first,
        second,
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    ))
    .expect("invariant: valid pair dimensions and subcycles")
}

type ConstantModel<A, B, R> = PairComponents<ConstantOutput<f64>, ConstantOutput<f64>, A, B, R>;

fn constant_pair<A, B, R>(
    first_to_second: A,
    second_to_first: B,
    relaxation: R,
) -> PartitionedPair<ConstantModel<A, B, R>, f64, 1, 1>
where
    ConstantModel<A, B, R>: PairModel<f64>,
{
    PartitionedPair::for_model(PairComponents::new(
        ConstantOutput { output: 2.0 },
        ConstantOutput { output: -3.0 },
        first_to_second,
        second_to_first,
        relaxation,
    ))
    .expect("invariant: valid pair dimensions and subcycles")
}

#[test]
fn caller_slice_dimension_failures_are_transactional() {
    let roles = [
        (FIRST_STATE, "first state"),
        (SECOND_STATE, "second state"),
        (FIRST_INPUT, "first input"),
        (SECOND_INPUT, "second input"),
    ];
    for (slot, role) in roles {
        for actual in [0, 2] {
            let mut caller = Caller::nominal();
            caller.0[slot] = vec![7.5; actual];
            let mut pair = constant_pair(IdentityTransfer, IdentityTransfer, FullRelaxation);

            let error = caller.rejection(&mut pair, instant(), window(1.0), 1);

            assert_eq!(
                error,
                CouplingError::Dimension {
                    role,
                    expected: 1,
                    actual
                }
            );
        }
    }
}

#[test]
fn time_overflow_is_transactional() {
    let start = Instant::new(Time::from_base(f64::MAX)).expect("invariant: finite instant");
    let mut pair = constant_pair(IdentityTransfer, IdentityTransfer, FullRelaxation);

    let error = Caller::nominal().rejection(&mut pair, start, window(f64::MAX), 2);

    assert_eq!(error, CouplingError::Time(TimeError::NonFinite));
}

#[test]
fn child_step_underflow_is_transactional() {
    let mut pair = PartitionedPair::<_, f64, 2, 1>::for_model(PairComponents::new(
        ConstantOutput { output: 2.0 },
        ConstantOutput { output: -3.0 },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    ))
    .expect("invariant: valid pair dimensions and subcycles");
    // Half the smallest subnormal rounds to zero, so the child step is invalid.
    let smallest_subnormal = f64::from_bits(1);

    let error = Caller::nominal().rejection(&mut pair, instant(), window(smallest_subnormal), 2);

    assert_eq!(
        error,
        CouplingError::Subcycle(SubcycleError::ChildStep(TimeError::NonPositiveStep))
    );
}

#[test]
fn transfer_failures_are_transactional_in_both_directions() {
    let expected = CouplingError::Transfer(TransferError::SourceIndex {
        index: 5,
        source_dimension: 1,
    });

    let mut forward = constant_pair(IndexTransfer::<5>, IdentityTransfer, FullRelaxation);
    let error = Caller::nominal().rejection(&mut forward, instant(), window(1.0), 2);
    assert_eq!(error, expected);

    let mut backward = constant_pair(IdentityTransfer, IndexTransfer::<5>, FullRelaxation);
    let error = Caller::nominal().rejection(&mut backward, instant(), window(1.0), 2);
    assert_eq!(error, expected);
}

#[test]
fn relaxation_failure_is_transactional() {
    let mut pair = constant_pair(IdentityTransfer, IdentityTransfer, RejectingRelaxation);

    let error = Caller::nominal().rejection(&mut pair, instant(), window(1.0), 2);

    assert_eq!(
        error,
        CouplingError::Relaxation(RelaxationError::NonFinite { index: 1 })
    );
}

#[test]
fn partition_failures_are_transactional_and_typed_by_partition() {
    let failing_advance = |substep| Faulty {
        advance_failure: Some(substep),
        ..Faulty::HEALTHY
    };
    let failing_export = Faulty {
        export_failure: true,
        ..Faulty::HEALTHY
    };

    let mut pair = faulty_pair::<3, 1>(failing_advance(2), Faulty::HEALTHY);
    let error = Caller::nominal().rejection(&mut pair, instant(), window(1.0), 2);
    assert_eq!(error, CouplingError::First(Fault::Advance { substep: 2 }));

    let mut pair = faulty_pair::<1, 2>(Faulty::HEALTHY, failing_advance(1));
    let error = Caller::nominal().rejection(&mut pair, instant(), window(1.0), 2);
    assert_eq!(error, CouplingError::Second(Fault::Advance { substep: 1 }));

    let mut pair = faulty_pair::<1, 1>(failing_export, Faulty::HEALTHY);
    let error = Caller::nominal().rejection(&mut pair, instant(), window(1.0), 2);
    assert_eq!(error, CouplingError::First(Fault::Export));

    let mut pair = faulty_pair::<1, 1>(Faulty::HEALTHY, failing_export);
    let error = Caller::nominal().rejection(&mut pair, instant(), window(1.0), 2);
    assert_eq!(error, CouplingError::Second(Fault::Export));
}
