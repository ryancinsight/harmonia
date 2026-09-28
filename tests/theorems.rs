//! Executable evidence for Phase 0 coupling theorems.

mod support;

use core::{convert::Infallible, fmt::Debug};
use std::{cell::RefCell, rc::Rc};

use athena_core::ConvergencePolicy;
use harmonia::{
    AitkenRelaxation, CouplingError, CouplingReport, FixedRelaxation, FullRelaxation,
    IdentityTransfer, PairComponents, PairModel, PairWorkspace, Partition, PartitionedPair,
    Substep,
};

use support::{
    ConstantOutput, CountingPartition, LastObserver, LinearPartition, Scaffold, euclidean_error,
    exact_interface, instant, linear_pair, solve, window,
};

fn linear_partition_for_step(partition: LinearPartition<f64>, step: u32) -> LinearPartition<f64> {
    let weight = f64::from(step);
    LinearPartition {
        source: weight * partition.source,
        gain: weight * partition.gain,
    }
}

type CountingPair = PartitionedPair<
    PairComponents<
        CountingPartition,
        CountingPartition,
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    >,
    f64,
    1,
    1,
>;

#[derive(Clone, Debug)]
struct RecordingAffinePartition {
    source: f64,
    gain: f64,
    inputs: Rc<RefCell<Vec<f64>>>,
}

impl Partition<f64> for RecordingAffinePartition {
    type Error = Infallible;
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
        _substep: Substep<f64>,
        state: &mut [f64],
        input: &[f64],
    ) -> Result<(), Self::Error> {
        self.inputs.borrow_mut().push(input[0]);
        state[0] = self.gain.mul_add(input[0], self.source);
        Ok(())
    }

    fn export(&self, state: &[f64], output: &mut [f64]) -> Result<(), Self::Error> {
        output.copy_from_slice(state);
        Ok(())
    }
}

type WindowValues = [[f64; 1]; 4];
const INITIAL_WINDOW: WindowValues = [[0.25], [-0.5], [0.0], [0.0]];

fn solve_linear_window<M>(
    pair: &mut PartitionedPair<M, f64, 1, 1>,
    values: &mut WindowValues,
    policy: &ConvergencePolicy<f64>,
) -> CouplingReport<f64>
where
    M: PairModel<f64>,
    <M::First as Partition<f64>>::Error: Debug,
    <M::Second as Partition<f64>>::Error: Debug,
{
    let [first_state, second_state, first_input, second_input] = values;
    pair.solve_window(
        instant(),
        window(0.5),
        first_state,
        second_state,
        first_input,
        second_input,
        policy,
        &mut LastObserver::default(),
    )
    .expect("the checkpointed linear map converges")
}

fn assert_exact_values(actual: [f64; 4], expected: [f64; 4]) {
    assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
}

fn assert_replayed_window(
    pair: &CountingPair,
    reports: [CouplingReport<f64>; 2],
    actual: WindowValues,
    stateless: WindowValues,
    expected: [f64; 4],
    steps: [u32; 2],
) {
    assert_eq!(reports[0].iterations, 3);
    assert_eq!(reports[0].residual_norm.to_bits(), 0.0_f64.to_bits());
    let actual = actual.map(|entry| entry[0]);
    assert_exact_values(actual, expected);
    assert_exact_values(actual, stateless.map(|entry| entry[0]));
    assert_eq!(pair.model().first().steps, steps[0]);
    assert_eq!(pair.model().second().steps, steps[1]);
}

#[test]
fn internal_partition_state_replays_one_fixed_map() {
    let first = LinearPartition {
        source: 0.5_f64,
        gain: 0.0,
    };
    let second = LinearPartition {
        source: -0.25_f64,
        gain: 0.125,
    };
    let model = PairComponents::new(
        CountingPartition {
            source: first.source,
            gain: first.gain,
            steps: 3,
        },
        CountingPartition {
            source: second.source,
            gain: second.gain,
            steps: 6,
        },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let mut pair = PartitionedPair::for_model(model).expect("invariant: valid pair dimensions");
    let mut actual = INITIAL_WINDOW;
    let policy = ConvergencePolicy::new(0.0, 0.0, 8).expect("invariant: valid policy");
    let first_report = solve_linear_window(&mut pair, &mut actual, &policy);

    let stateless_model = PairComponents::new(
        linear_partition_for_step(first, 4),
        linear_partition_for_step(second, 7),
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let mut stateless_pair =
        PartitionedPair::for_model(stateless_model).expect("invariant: valid pair dimensions");
    let mut stateless = INITIAL_WINDOW;
    let first_stateless_report = solve_linear_window(&mut stateless_pair, &mut stateless, &policy);

    // Successors 4 and 7 reach exact (-53/64, 5/4) and repeat it on evaluation three.
    assert_replayed_window(
        &pair,
        [first_report, first_stateless_report],
        actual,
        stateless,
        [1.25, -0.828_125, -0.828_125, 1.25],
        [4, 7],
    );

    *stateless_pair.model_mut().first_mut() = linear_partition_for_step(first, 5);
    *stateless_pair.model_mut().second_mut() = linear_partition_for_step(second, 8);

    let second_report = solve_linear_window(&mut pair, &mut actual, &policy);
    let second_stateless_report = solve_linear_window(&mut stateless_pair, &mut stateless, &policy);

    // Successors 5 and 8 similarly yield the exact point (-37/64, 5/2).
    assert_replayed_window(
        &pair,
        [second_report, second_stateless_report],
        actual,
        stateless,
        [2.5, -0.578_125, -0.578_125, 2.5],
        [5, 8],
    );
}

#[test]
fn contraction_residual_bounds_fixed_point_error() {
    let first = LinearPartition {
        source: 1.25_f64,
        gain: 0.2,
    };
    let second = LinearPartition {
        source: -0.5_f64,
        gain: -0.3,
    };
    let mut pair = linear_pair::<f64, 2, 3>(first, second);
    let mut scaffold = Scaffold::new(0.4, -0.1);
    let policy = ConvergencePolicy::new(1.0e-12, 1.0e-12, 64).expect("invariant: valid policy");
    let mut observer = LastObserver::default();

    let report = solve(&mut pair, 0.5, &policy, &mut scaffold, &mut observer)
        .expect("contractive pair converges");

    let exact = exact_interface(0.4, -0.1, 0.5, first, second);
    let actual = [scaffold.first_input[0], scaffold.second_input[0]];
    let contraction = (0.5 * first.gain).abs().max((0.5 * second.gain).abs());
    let rounding = 32.0 * f64::EPSILON * (1.0 + exact[0].abs() + exact[1].abs());
    let theorem_bound = report.residual_norm / (1.0 - contraction) + rounding;

    assert!(euclidean_error(actual, exact) <= theorem_bound);
    assert!((scaffold.first_state[0] - exact[1]).abs() <= theorem_bound);
    assert!((scaffold.second_state[0] - exact[0]).abs() <= theorem_bound);
    assert_eq!(observer.count, report.iterations);
}

#[test]
fn nonconvergence_is_transactional() {
    let model = PairComponents::new(
        ConstantOutput { output: 2.0_f64 },
        ConstantOutput { output: -3.0_f64 },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("valid subcycles");
    let mut first_state = [11.0];
    let mut second_state = [12.0];
    let mut first_input = [13.0];
    let mut second_input = [14.0];
    let before = (first_state, second_state, first_input, second_input);
    let policy = ConvergencePolicy::new(0.0, 0.0, 1).expect("invariant: one iteration is valid");

    let result = pair.solve_window(
        instant(),
        window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut LastObserver::default(),
    );

    assert!(matches!(
        result,
        Err(CouplingError::NotConverged { iterations: 1, .. })
    ));
    assert_eq!(
        (first_state, second_state, first_input, second_input),
        before
    );
}

#[test]
fn failed_aitken_window_cannot_seed_the_next_window() {
    let first_inputs = Rc::new(RefCell::new(Vec::new()));
    let second_inputs = Rc::new(RefCell::new(Vec::new()));
    let model = PairComponents::new(
        RecordingAffinePartition {
            source: 1.0,
            gain: 0.5,
            inputs: Rc::clone(&first_inputs),
        },
        RecordingAffinePartition {
            source: 2.0,
            gain: 0.25,
            inputs: Rc::clone(&second_inputs),
        },
        IdentityTransfer,
        IdentityTransfer,
        AitkenRelaxation::new(0.05, 1.5, 1.0e-12).expect("invariant: valid Aitken configuration"),
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair =
        PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("invariant: valid subcycles");
    let policy = ConvergencePolicy::new(0.0, 0.0, 2).expect("invariant: two iterations are valid");
    let mut first_state = [0.0];
    let mut second_state = [0.0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];

    let first_result = pair.solve_window(
        instant(),
        window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut LastObserver::default(),
    );
    assert!(matches!(
        first_result,
        Err(CouplingError::NotConverged { iterations: 2, .. })
    ));

    first_input[0] = 10.0;
    second_input[0] = 10.0;
    let second_result = pair.solve_window(
        instant(),
        window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut LastObserver::default(),
    );
    assert!(matches!(
        second_result,
        Err(CouplingError::NotConverged { iterations: 2, .. })
    ));

    assert_eq!(first_inputs.borrow().as_slice(), &[0.0, 2.0, 10.0, 4.5]);
    assert_eq!(second_inputs.borrow().as_slice(), &[0.0, 1.0, 10.0, 6.0]);
}

#[test]
fn relaxation_weight_cannot_manufacture_convergence() {
    let relaxation =
        FixedRelaxation::new(1.0e-12_f64).expect("invariant: positive unit-interval weight");
    let model = PairComponents::new(
        ConstantOutput { output: 1.0_f64 },
        ConstantOutput { output: 1.0_f64 },
        IdentityTransfer,
        IdentityTransfer,
        relaxation,
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, workspace).expect("valid subcycles");
    let policy = ConvergencePolicy::new(1.0e-6, 0.0, 2).expect("invariant: valid tolerance");
    let mut first_state = [0.0];
    let mut second_state = [0.0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];

    let result = pair.solve_window(
        instant(),
        window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut LastObserver::default(),
    );

    let Err(CouplingError::NotConverged { residual_norm, .. }) = result else {
        panic!("raw defect must remain above tolerance");
    };
    assert!(residual_norm > 1.0);
}
