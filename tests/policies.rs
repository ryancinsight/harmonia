//! Transfer, relaxation, layout, and workspace policy boundaries.

mod support;

use core::mem::size_of;
use std::borrow::Cow;

use harmonia::{
    FixedRelaxation, FullRelaxation, IdentityTransfer, IndexTransfer, InvalidRelaxation,
    PairComponents, PairWorkspace, Transfer, TransferError, WorkspaceError,
};

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
