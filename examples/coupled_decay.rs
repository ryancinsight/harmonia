//! Coupled two-rate decay over one heterogeneous-subcycle window.

use core::convert::Infallible;

use aequitas::systems::si::quantities::Time;
use athena_core::{ConvergencePolicy, NoObserver};
use harmonia::{
    FullRelaxation, IdentityTransfer, PairComponents, PairWorkspace, Partition, PartitionedPair,
    Substep,
};
use horae::time::{Instant, StepSize};

#[derive(Clone, Copy)]
struct Decay {
    rate: f64,
}

impl Partition<f64> for Decay {
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
        substep: Substep<f64>,
        state: &mut [f64],
        input: &[f64],
    ) -> Result<(), Self::Error> {
        let step = *substep.size().as_time().as_base();
        state[0] += step * (-self.rate * state[0] + input[0]);
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        output.copy_from_slice(state);
        Ok(())
    }
}

// Dynamic error erasure is confined to this non-hot example entry boundary.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = PairComponents::new(
        Decay { rate: 1.0 },
        Decay { rate: 2.0 },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model)?;
    let mut coupling = PartitionedPair::<_, f64, 2, 4>::new(model, workspace)?;
    let start = Instant::new(Time::from_base(0.0))?;
    let window = StepSize::new(Time::from_base(0.1))?;
    let policy = ConvergencePolicy::new(1.0e-12, 1.0e-12, 32)?;
    let mut first_state = [1.0];
    let mut second_state = [0.5];
    let mut first_input = [0.5];
    let mut second_input = [1.0];

    let report = coupling.solve_window(
        start,
        window,
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut NoObserver,
    )?;

    println!(
        "iterations={} residual={:.3e} first={:.6} second={:.6}",
        report.iterations, report.residual_norm, first_state[0], second_state[0]
    );
    Ok(())
}
