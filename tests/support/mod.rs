#![allow(dead_code)]

use core::convert::Infallible;

use aequitas::systems::si::quantities::Time;
use athena_core::{IterationObserver, IterationState};
use eunomia::{FloatElement, NumericElement, RealField};
use harmonia::{Partition, Substep};
use horae::time::{Instant, StepSize};

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
pub struct ConstantOutput<T> {
    pub output: T,
}

impl<T> Partition<T> for ConstantOutput<T>
where
    T: RealField,
{
    type Error = Infallible;

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
