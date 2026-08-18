//! Transfer, relaxation, layout, and workspace policy boundaries.

mod support;

use core::mem::size_of;
use std::borrow::Cow;

use harmonia::{
    FixedRelaxation, FullRelaxation, IdentityTransfer, IndexTransfer, InvalidRelaxation,
    PairComponents, PairWorkspace, PartitionedPair, Relaxation, RelaxationError, Transfer,
    TransferError, WorkspaceError,
};

use athena_core::ConvergencePolicy;
use support::Dimensions;

#[test]
fn identity_transfer_preserves_borrow_and_pointer() {
    let source = [1_u32, 2, 3];
    let mut scratch = [0_u32; 3];

    let result = IdentityTransfer
        .transfer(&source, &mut scratch)
        .expect("identity transfer is infallible");

    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(result.as_ptr(), source.as_ptr());
    assert_eq!(result.as_ref(), source);
}

#[test]
fn identity_transfer_rejects_mismatched_scratch_dimension() {
    let source = [1_u32, 2, 3];
    let mut scratch = [0_u32; 2];

    assert_eq!(
        IdentityTransfer.transfer(&source, &mut scratch),
        Err(TransferError::Dimension {
            expected: 3,
            actual: 2,
        })
    );
}

#[test]
fn const_index_transfer_selects_without_ownership() {
    let source = [3_u32, 5, 8];
    let mut scratch = [0_u32; 1];

    let result = IndexTransfer::<1>
        .transfer(&source, &mut scratch)
        .expect("index one exists");

    assert!(matches!(result, Cow::Borrowed(_)));
    assert_eq!(result.as_ref(), &[5]);
    assert_eq!(result.as_ptr(), scratch.as_ptr());
}

#[test]
fn const_index_transfer_reports_exact_boundary() {
    let source = [3_u32, 5, 8];
    let mut scratch = [0_u32; 1];

    assert_eq!(
        IndexTransfer::<3>.transfer(&source, &mut scratch),
        Err(TransferError::SourceIndex {
            index: 3,
            source_dimension: 3,
        })
    );
}

#[test]
fn static_policies_have_zero_runtime_footprint() {
    assert_eq!(size_of::<IdentityTransfer>(), 0);
    assert_eq!(size_of::<IndexTransfer<7>>(), 0);
    assert_eq!(size_of::<FullRelaxation>(), 0);
}

#[derive(Debug, Default, PartialEq, Eq)]
struct StatefulRelaxation {
    updates: usize,
}

impl Relaxation<f64> for StatefulRelaxation {
    fn update_pair(
        &mut self,
        first_current: &mut [f64],
        first_candidate: &[f64],
        second_current: &mut [f64],
        second_candidate: &[f64],
    ) -> Result<(), RelaxationError> {
        self.updates += 1;
        FullRelaxation.update_pair(
            first_current,
            first_candidate,
            second_current,
            second_candidate,
        )
    }
}

#[test]
fn pair_relaxation_retains_state_across_iterations() {
    let model = PairComponents::new(
        support::ConstantOutput { output: 1.0 },
        support::ConstantOutput { output: 1.0 },
        IdentityTransfer,
        IdentityTransfer,
        StatefulRelaxation::default(),
    );
    let workspace = PairWorkspace::for_model(&model).expect("invariant: compatible dimensions");
    let mut pair = PartitionedPair::<_, f64, 1, 1>::new(model, workspace)
        .expect("invariant: positive subcycle ratios");
    let policy = ConvergencePolicy::new(0.0, 0.0, 2).expect("invariant: valid policy");
    let mut first_state = [0.0];
    let mut second_state = [0.0];
    let mut first_input = [0.0];
    let mut second_input = [0.0];

    pair.solve_window(
        support::instant(),
        support::window(1.0),
        &mut first_state,
        &mut second_state,
        &mut first_input,
        &mut second_input,
        &policy,
        &mut support::LastObserver::default(),
    )
    .expect("zero-output pair converges after the first update");

    let (model, _) = pair.into_parts();
    let (_, _, _, _, relaxation) = model.into_parts();
    assert_eq!(relaxation.updates, 1);
}

#[test]
fn fixed_relaxation_rejects_every_invalid_interval_boundary() {
    for weight in [
        f64::NAN,
        f64::INFINITY,
        -f64::EPSILON,
        0.0,
        1.0 + f64::EPSILON,
    ] {
        assert_eq!(
            FixedRelaxation::new(weight),
            Err(InvalidRelaxation::OutsideUnitInterval)
        );
    }
    assert_eq!(
        FixedRelaxation::new(0.5_f64)
            .expect("one half is valid")
            .weight()
            .to_bits(),
        0.5_f64.to_bits()
    );
}

#[test]
fn built_in_relaxation_keeps_both_interfaces_on_late_failure() {
    let mut first_current = [2.0_f64];
    let mut second_current = [3.0_f64];
    let first_candidate = [4.0_f64];
    let second_candidate = [f64::NAN];
    let mut fixed = FixedRelaxation::new(0.5).expect("invariant: valid weight");

    assert_eq!(
        fixed.update_pair(
            &mut first_current,
            &first_candidate,
            &mut second_current,
            &second_candidate,
        ),
        Err(RelaxationError::NonFinite { index: 1 })
    );
    assert_eq!(first_current[0].to_bits(), 2.0_f64.to_bits());
    assert_eq!(second_current[0].to_bits(), 3.0_f64.to_bits());

    let mut full_first = [2.0_f64];
    let mut full_second = [3.0_f64];
    assert_eq!(
        FullRelaxation.update_pair(
            &mut full_first,
            &first_candidate,
            &mut full_second,
            &second_candidate,
        ),
        Err(RelaxationError::NonFinite { index: 1 })
    );
    assert_eq!(full_first[0].to_bits(), 2.0_f64.to_bits());
    assert_eq!(full_second[0].to_bits(), 3.0_f64.to_bits());
}

#[test]
fn workspace_rejects_transfer_dimension_mismatch() {
    let model = PairComponents::new(
        Dimensions {
            state: 1,
            input: 2,
            output: 1,
        },
        Dimensions {
            state: 1,
            input: 1,
            output: 1,
        },
        IdentityTransfer,
        IdentityTransfer,
        FullRelaxation,
    );

    assert!(matches!(
        PairWorkspace::for_model(&model),
        Err(WorkspaceError::TransferDimension {
            role: "second-to-first transfer",
            transfer: 1,
            input: 2,
        })
    ));
}
