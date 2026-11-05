//! A workspace must match the model actually used by the coupling pair.

use core::convert::Infallible;
use std::{
    borrow::Cow,
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

use aequitas::systems::si::quantities::Time;
use athena_core::{ConvergencePolicy, NoObserver};
use harmonia::{
    CouplingError, FullRelaxation, IdentityTransfer, IndexTransfer, PairComponents, PairWorkspace,
    Partition, PartitionedPair, Substep, Transfer, TransferError,
};
use horae::time::{Instant, StepSize};

/// Linear scalar dynamics with one or two physical exports.
#[derive(Clone, Copy)]
struct LinearPartition {
    gain: f64,
    dual_output: bool,
}

impl Partition<f64> for LinearPartition {
    type Error = Infallible;
    type Checkpoint = ();

    fn checkpoint(&self) -> Self::Checkpoint {}
    fn restore(&mut self, &(): &Self::Checkpoint) {}
    fn state_dimension(&self) -> usize {
        1
    }
    fn input_dimension(&self) -> usize {
        1
    }
    fn output_dimension(&self) -> usize {
        if self.dual_output { 2 } else { 1 }
    }

    fn advance(
        &mut self,
        step: Substep<f64>,
        state: &mut [f64],
        input: &[f64],
    ) -> Result<(), Self::Error> {
        state[0] += *step.size().as_time().as_base() * (1.0 + self.gain * input[0]);
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        output[0] = state[0];
        if self.dual_output {
            output[1] = 2.0 * state[0];
        }
        Ok(())
    }
}

fn instant() -> Instant<f64> {
    Instant::new(Time::from_base(0.0)).expect("zero is a valid start")
}

fn window() -> StepSize<f64> {
    StepSize::new(Time::from_base(1.0)).expect("one is a valid window")
}

type TwoExportModel = PairComponents<
    LinearPartition,
    LinearPartition,
    IndexTransfer<1>,
    IdentityTransfer,
    FullRelaxation,
>;

fn two_export_model() -> TwoExportModel {
    PairComponents::new(
        LinearPartition {
            gain: 0.5,
            dual_output: true,
        },
        LinearPartition {
            gain: 0.25,
            dual_output: false,
        },
        IndexTransfer::<1>,
        IdentityTransfer,
        FullRelaxation,
    )
}

#[test]
fn validated_matching_workspace_solves_the_coupled_linear_system() {
    let model = two_export_model();
    let workspace = PairWorkspace::for_model(&model).expect("valid two-output model");
    let mut pair =
        PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("positive subcycles");
    let mut first_state = [1.0];
    let mut second_state = [2.0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];
    let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");

    let report = pair
        .solve_window(
            instant(),
            window(),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        )
        .expect("contracting system converges");
    assert!(report.iterations > 1);
    assert!((first_state[0] - 14.0 / 3.0).abs() < 1e-9);
    assert!((second_state[0] - 16.0 / 3.0).abs() < 1e-9);
    assert!((first_input[0] - 16.0 / 3.0).abs() < 1e-9);
    assert!((second_input[0] - 28.0 / 3.0).abs() < 1e-9);
}

#[test]
fn foreign_workspace_returns_dimension_error_without_invoking_export() {
    let smaller_model = PairComponents::new(
        LinearPartition {
            gain: 0.5,
            dual_output: false,
        },
        LinearPartition {
            gain: 0.25,
            dual_output: false,
        },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let foreign = PairWorkspace::for_model(&smaller_model).expect("valid one-output workspace");
    let mut pair = PartitionedPair::<_, f64, 1, 1>::new(two_export_model(), foreign)
        .expect("positive subcycles");
    let mut first_state = [1.0];
    let mut second_state = [2.0];
    let mut first_input = [-0.0_f64];
    let mut second_input = [0.0];
    let before = (first_state, second_state, first_input, second_input);
    let before_bits = [
        first_state[0],
        second_state[0],
        first_input[0],
        second_input[0],
    ]
    .map(f64::to_bits);
    let policy = ConvergencePolicy::new(1e-12, 1e-12, 100).expect("valid policy");

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        pair.solve_window(
            instant(),
            window(),
            &mut first_state,
            &mut second_state,
            &mut first_input,
            &mut second_input,
            &policy,
            &mut NoObserver,
        )
    }))
    .expect("an incompatible workspace must return an error, not panic");
    match outcome {
        Err(CouplingError::Dimension {
            role,
            expected,
            actual,
        }) => {
            assert!(role.contains("first") && role.contains("output"));
            assert_eq!((expected, actual), (2, 1));
        }
        other => panic!("expected first-output dimension failure, got {other:?}"),
    }
    assert_eq!(
        (first_state, second_state, first_input, second_input),
        before
    );
    assert_eq!(
        [
            first_state[0],
            second_state[0],
            first_input[0],
            second_input[0]
        ]
        .map(f64::to_bits),
        before_bits
    );
}

#[derive(Clone)]
struct SizedLinear {
    state: usize,
    input: usize,
    output: usize,
    gain: f64,
    calls: Rc<Cell<usize>>,
    change_on_advance: Option<Rc<Cell<usize>>>,
}

impl Partition<f64> for SizedLinear {
    type Error = Infallible;
    type Checkpoint = Option<usize>;

    fn checkpoint(&self) -> Self::Checkpoint {
        self.calls.set(self.calls.get() + 1);
        self.change_on_advance.as_ref().map(|value| value.get())
    }
    fn restore(&mut self, checkpoint: &Self::Checkpoint) {
        self.calls.set(self.calls.get() + 1);
        if let (Some(value), Some(saved)) = (&self.change_on_advance, checkpoint) {
            value.set(*saved);
        }
    }
    fn state_dimension(&self) -> usize {
        self.state
    }
    fn input_dimension(&self) -> usize {
        self.input
    }
    fn output_dimension(&self) -> usize {
        self.output
    }

    fn advance(
        &mut self,
        step: Substep<f64>,
        state: &mut [f64],
        input: &[f64],
    ) -> Result<(), Self::Error> {
        self.calls.set(self.calls.get() + 1);
        if let Some(value) = &self.change_on_advance {
            value.set(2);
        }
        let increment =
            *step.size().as_time().as_base() * (1.0 + self.gain * input.iter().sum::<f64>());
        for value in state {
            *value += increment;
        }
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        self.calls.set(self.calls.get() + 1);
        for (index, value) in output.iter_mut().enumerate() {
            *value = state[0] * f64::from(u32::try_from(index + 1).expect("test dimensions fit"));
        }
        Ok(())
    }
}

#[derive(Clone)]
enum Directional {
    Identity,
    Select,
    Replicate,
    Truncated,
    Adaptive {
        dimension: Rc<Cell<usize>>,
        calls: Rc<Cell<usize>>,
    },
}

impl Transfer<f64> for Directional {
    fn destination_dimension(&self, source_dimension: usize) -> usize {
        match self {
            Self::Identity | Self::Truncated => source_dimension,
            Self::Select => 1,
            Self::Replicate => 2,
            Self::Adaptive { dimension, .. } => dimension.get(),
        }
    }

    fn transfer<'a>(
        &self,
        source: &'a [f64],
        scratch: &'a mut [f64],
    ) -> Result<Cow<'a, [f64]>, TransferError> {
        match self {
            Self::Identity => IdentityTransfer.transfer(source, scratch),
            Self::Select => IndexTransfer::<1>.transfer(source, scratch),
            Self::Replicate => {
                if scratch.len() != 2 {
                    return Err(TransferError::Dimension {
                        expected: 2,
                        actual: scratch.len(),
                    });
                }
                let Some(&value) = source.first() else {
                    return Err(TransferError::SourceIndex {
                        index: 0,
                        source_dimension: source.len(),
                    });
                };
                scratch.copy_from_slice(&[value, value]);
                Ok(Cow::Borrowed(scratch))
            }
            // An adversarial third-party projection claims the full input
            // length but actually returns a shorter, borrowed data slice.
            Self::Truncated => Ok(Cow::Borrowed(&source[..source.len().saturating_sub(1)])),
            Self::Adaptive { dimension, calls } => {
                calls.set(calls.get() + 1);
                for (index, value) in scratch[..dimension.get()].iter_mut().enumerate() {
                    *value = source[0]
                        * f64::from(u32::try_from(index + 1).expect("test dimensions fit"));
                }
                Ok(Cow::Borrowed(scratch))
            }
        }
    }
}

fn sized_partition(gain: f64, calls: &Rc<Cell<usize>>) -> SizedLinear {
    SizedLinear {
        state: 1,
        input: 1,
        output: 1,
        gain,
        calls: Rc::clone(calls),
        change_on_advance: None,
    }
}

fn unit_workspace() -> PairWorkspace<f64> {
    let calls = Rc::new(Cell::new(0));
    let model = PairComponents::new(
        sized_partition(0.5, &calls),
        sized_partition(0.25, &calls),
        Directional::Identity,
        Directional::Identity,
        FullRelaxation,
    );
    PairWorkspace::for_model(&model).expect("a compatible unit workspace")
}

#[path = "workspace_contract/matrices.rs"]
mod matrices;
