use core::convert::Infallible;

use aequitas::systems::si::quantities::Time;
use athena_core::{ConvergencePolicy, IterationObserver, IterationState};
use eunomia::{FloatElement, NumericElement, RealField};
use harmonia::{
    CouplingError, CouplingReport, FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace,
    Partition, PartitionedPair, Substep,
};
use horae::time::{Instant, StepSize};

/// Emit the boilerplate `Partition` plumbing that every test partition shares.
///
/// The `unit` arm covers partitions with no internal state; the `steps` arm
/// covers the counting partition whose checkpoint is its step counter.
macro_rules! partition_plumbing {
    (unit, $state:expr, $input:expr, $output:expr) => {
        type Checkpoint = ();

        fn checkpoint(&self) -> Self::Checkpoint {}

        fn restore(&mut self, _checkpoint: &Self::Checkpoint) {}

        fn state_dimension(&self) -> usize {
            $state
        }

        fn input_dimension(&self) -> usize {
            $input
        }

        fn output_dimension(&self) -> usize {
            $output
        }
    };
    (steps, $state:expr, $input:expr, $output:expr) => {
        type Checkpoint = u32;

        fn checkpoint(&self) -> Self::Checkpoint {
            self.steps
        }

        fn restore(&mut self, checkpoint: &Self::Checkpoint) {
            self.steps = *checkpoint;
        }

        fn state_dimension(&self) -> usize {
            $state
        }

        fn input_dimension(&self) -> usize {
            $input
        }

        fn output_dimension(&self) -> usize {
            $output
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub struct LinearPartition<T> {
    pub source: T,
    pub gain: T,
}

impl<T> Partition<T> for LinearPartition<T>
where
    T: RealField,
{
    type Error = Infallible;
    partition_plumbing!(unit, 1, 1, 1);

    fn advance(
        &mut self,
        substep: Substep<T>,
        state: &mut [T],
        input: &[T],
    ) -> Result<(), Self::Error> {
        let step = *substep.size().as_time().as_base();
        state[0] += step * (self.source + self.gain * input[0]);
        Ok(())
    }

    fn export(&self, state: &[T], output: &mut [T]) -> Result<(), Self::Error> {
        output.copy_from_slice(state);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CountingPartition {
    pub source: f64,
    pub gain: f64,
    pub steps: u32,
}

impl Partition<f64> for CountingPartition {
    type Error = Infallible;
    partition_plumbing!(steps, 1, 1, 1);

    fn advance(
        &mut self,
        substep: Substep<f64>,
        state: &mut [f64],
        input: &[f64],
    ) -> Result<(), Self::Error> {
        self.steps = self
            .steps
            .checked_add(1)
            .expect("invariant: the test iteration budget fits in u32");
        let step = *substep.size().as_time().as_base();
        state[0] += step * f64::from(self.steps) * (self.source + self.gain * input[0]);
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        output.copy_from_slice(state);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConstantOutput<T> {
    pub output: T,
}

impl<T> Partition<T> for ConstantOutput<T>
where
    T: RealField,
{
    type Error = Infallible;
    partition_plumbing!(unit, 1, 1, 1);

    fn advance(
        &mut self,
        _substep: Substep<T>,
        state: &mut [T],
        _input: &[T],
    ) -> Result<(), Self::Error> {
        state[0] = self.output;
        Ok(())
    }

    fn export(&self, state: &[T], output: &mut [T]) -> Result<(), Self::Error> {
        output.copy_from_slice(state);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Dimensions {
    pub state: usize,
    pub input: usize,
    pub output: usize,
}

impl Partition<f64> for Dimensions {
    type Error = Infallible;
    type Checkpoint = ();

    fn checkpoint(&self) -> Self::Checkpoint {}

    fn restore(&mut self, _checkpoint: &Self::Checkpoint) {}

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
        _substep: Substep<f64>,
        _state: &mut [f64],
        _input: &[f64],
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn export(&self, _state: &[f64], _output: &mut [f64]) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LastObserver<T> {
    pub sample: Option<IterationState<T>>,
    pub count: usize,
}

impl<T> IterationObserver<T> for LastObserver<T> {
    fn observe(&mut self, state: IterationState<T>) {
        self.sample = Some(state);
        self.count += 1;
    }
}

pub fn instant<T>() -> Instant<T>
where
    T: FloatElement,
{
    Instant::new(Time::from_base(<T as NumericElement>::ZERO))
        .expect("invariant: zero is a finite instant")
}

pub fn window<T>(value: T) -> StepSize<T>
where
    T: FloatElement,
{
    StepSize::new(Time::from_base(value)).expect("invariant: test window is positive and finite")
}

/// The four single-element caller buffers of one linear-pair window solve.
#[derive(Clone, Copy, Debug)]
pub struct Scaffold<T> {
    pub first_state: [T; 1],
    pub second_state: [T; 1],
    pub first_input: [T; 1],
    pub second_input: [T; 1],
}

impl<T> Scaffold<T>
where
    T: NumericElement,
{
    /// Start a scaffold from a pair of scalar initial states and zero inputs.
    pub fn new(first_state: T, second_state: T) -> Self {
        Self {
            first_state: [first_state],
            second_state: [second_state],
            first_input: [<T as NumericElement>::ZERO],
            second_input: [<T as NumericElement>::ZERO],
        }
    }
}

/// The identity-transfer, full-relaxation linear model shared by most tests.
pub type LinearModel<T> = PairComponents<
    LinearPartition<T>,
    LinearPartition<T>,
    IdentityTransfer,
    IdentityTransfer,
    FullRelaxation,
>;

/// Build a single-element linear pair with the requested subcycle counts.
pub fn linear_pair<T, const FIRST: usize, const SECOND: usize>(
    first: LinearPartition<T>,
    second: LinearPartition<T>,
) -> PartitionedPair<LinearModel<T>, T, FIRST, SECOND>
where
    T: RealField,
{
    let model = PairComponents::new(
        first,
        second,
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    PartitionedPair::new(model, workspace).expect("invariant: valid subcycle plan")
}

/// Advance a linear `pair` by one window, reading its state through `scaffold`.
pub fn solve<T, O, const FIRST: usize, const SECOND: usize>(
    pair: &mut PartitionedPair<LinearModel<T>, T, FIRST, SECOND>,
    span: T,
    policy: &ConvergencePolicy<T>,
    scaffold: &mut Scaffold<T>,
    observer: &mut O,
) -> Result<CouplingReport<T>, CouplingError<T, Infallible, Infallible>>
where
    T: RealField,
    O: IterationObserver<T>,
{
    pair.solve_window(
        instant(),
        window(span),
        &mut scaffold.first_state,
        &mut scaffold.second_state,
        &mut scaffold.first_input,
        &mut scaffold.second_input,
        policy,
        observer,
    )
}

/// Build and solve a single linear pair in one step.
pub fn solve_linear_pair<T, const FIRST: usize, const SECOND: usize>(
    first: LinearPartition<T>,
    second: LinearPartition<T>,
    span: T,
    policy: &ConvergencePolicy<T>,
    scaffold: &mut Scaffold<T>,
) -> Result<CouplingReport<T>, CouplingError<T, Infallible, Infallible>>
where
    T: RealField,
{
    let mut pair = linear_pair::<T, FIRST, SECOND>(first, second);
    solve(
        &mut pair,
        span,
        policy,
        scaffold,
        &mut LastObserver::default(),
    )
}

pub fn exact_interface(
    first_initial: f64,
    second_initial: f64,
    window: f64,
    first: LinearPartition<f64>,
    second: LinearPartition<f64>,
) -> [f64; 2] {
    let first_constant = first_initial + window * first.source;
    let second_constant = second_initial + window * second.source;
    let first_gain = window * first.gain;
    let second_gain = window * second.gain;
    let denominator = 1.0 - first_gain * second_gain;
    let first_input = (second_constant + second_gain * first_constant) / denominator;
    let second_input = first_constant + first_gain * first_input;
    [first_input, second_input]
}

pub fn euclidean_error(actual: [f64; 2], expected: [f64; 2]) -> f64 {
    let first = actual[0] - expected[0];
    let second = actual[1] - expected[1];
    first.mul_add(first, second * second).sqrt()
}

#[cfg(test)]
mod scaffolding_tests {
    use super::*;

    // Every integration test compiles this module into its own crate, so this
    // test is the single place that exercises the whole scaffolding. Without it
    // each crate would leave the helpers it does not use reported as dead code.
    #[test]
    fn scaffolding_is_self_consistent() {
        let first = LinearPartition {
            source: 1.0_f64,
            gain: 0.0,
        };
        let second = LinearPartition {
            source: -2.0_f64,
            gain: 0.0,
        };
        let mut scaffold = Scaffold::new(0.25_f64, 0.75_f64);
        let policy = ConvergencePolicy::new(0.0, 0.0, 4).expect("invariant: valid policy");

        let report = solve_linear_pair::<f64, 2, 3>(first, second, 0.5, &policy, &mut scaffold)
            .expect("constant linear pair converges");
        let exact = exact_interface(0.25, 0.75, 0.5, first, second);
        let solved = [scaffold.first_input[0], scaffold.second_input[0]];
        assert!(report.residual_norm.is_finite());
        assert!(euclidean_error(solved, exact).is_finite());

        let counting = CountingPartition {
            source: 0.0,
            gain: 0.0,
            steps: 0,
        };
        assert_eq!(counting.state_dimension(), 1);
        let constant = ConstantOutput { output: 1.0_f64 };
        assert_eq!(constant.input_dimension(), 1);
        let dimensions = Dimensions {
            state: 2,
            input: 3,
            output: 4,
        };
        assert_eq!(dimensions.output_dimension(), 4);

        let mut pair = linear_pair::<f64, 1, 1>(first, second);
        let mut observer = LastObserver::<f64>::default();
        solve(&mut pair, 0.5, &policy, &mut scaffold, &mut observer)
            .expect("counting pair converges");
        assert!(observer.count >= 1);
    }
}
