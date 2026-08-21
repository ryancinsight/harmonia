//! Executable evidence for the typed physical-field boundary.

use aequitas::Quantity;
use aequitas::systems::si::{dimensions, quantities::Length};
use harmonia::{FieldEnvelope, FieldError, GridGeometry};
use horae::time::Instant;
use proptest::prelude::*;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn geometry<T>() -> GridGeometry<T, 2>
where
    T: eunomia::RealField,
{
    GridGeometry::try_new(
        [2, 2],
        [Length::from_base(T::ONE), Length::from_base(T::ONE)],
        [Length::from_base(T::ZERO), Length::from_base(T::ZERO)],
        [[T::ONE, T::ZERO], [T::ZERO, T::ONE]],
    )
    .expect("invariant: identity geometry is valid")
}

#[test]
fn valid_envelope_borrows_quantity_values() {
    let values = [
        Quantity::<f64, dimensions::Intensity>::from_base(1.0),
        Quantity::from_base(2.0),
        Quantity::from_base(3.0),
        Quantity::from_base(4.0),
    ];
    let region = Region::new(ALLOCATOR);
    let field = FieldEnvelope::try_new(
        &values,
        geometry(),
        Instant::new(aequitas::systems::si::quantities::Time::from_base(0.0))
            .expect("invariant: zero is finite"),
    )
    .expect("invariant: value count matches geometry");
    let change = region.change();

    assert_eq!(field.values().as_ptr(), values.as_ptr());
    assert_eq!(field.values()[2].into_base().to_bits(), 3.0_f64.to_bits());
    assert_eq!(field.geometry().value_count(), 4);
    assert_eq!(
        field.time().into_time().into_base().to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(change.allocations, 0);
    assert_eq!(change.reallocations, 0);
    assert_eq!(change.deallocations, 0);
}

#[test]
fn rejects_invalid_geometry_and_value_count() {
    let identity = [[1.0, 0.0], [0.0, 1.0]];
    let spacing = [Length::from_base(1.0), Length::from_base(1.0)];
    let origin = [Length::from_base(0.0), Length::from_base(0.0)];

    assert_eq!(
        GridGeometry::try_new([0, 2], spacing, origin, identity),
        Err(FieldError::ZeroExtent { axis: 0 })
    );
    assert_eq!(
        GridGeometry::try_new(
            [2, 2],
            [Length::from_base(0.0), Length::from_base(1.0)],
            origin,
            identity,
        ),
        Err(FieldError::InvalidSpacing { axis: 0 })
    );
    assert_eq!(
        GridGeometry::try_new(
            [2, 2],
            [Length::from_base(-1.0), Length::from_base(1.0)],
            origin,
            identity,
        ),
        Err(FieldError::InvalidSpacing { axis: 0 })
    );
    assert_eq!(
        GridGeometry::try_new(
            [2, 2],
            [Length::from_base(f64::INFINITY), Length::from_base(1.0)],
            origin,
            identity,
        ),
        Err(FieldError::InvalidSpacing { axis: 0 })
    );
    assert_eq!(
        GridGeometry::try_new(
            [2, 2],
            spacing,
            [Length::from_base(f64::NAN), Length::from_base(0.0)],
            identity,
        ),
        Err(FieldError::NonFiniteOrigin { axis: 0 })
    );
    assert_eq!(
        GridGeometry::try_new([2, 2], spacing, origin, [[1.0, 0.0], [0.25, 1.0]],),
        Err(FieldError::NonOrthonormalDirection { row: 0, column: 1 })
    );

    let values = [Quantity::<f64, dimensions::Intensity>::from_base(1.0)];
    let result = FieldEnvelope::try_new(
        &values,
        geometry(),
        Instant::new(aequitas::systems::si::quantities::Time::from_base(0.0))
            .expect("invariant: zero is finite"),
    );
    assert_eq!(
        result,
        Err(FieldError::ValueCount {
            expected: 4,
            actual: 1,
        })
    );
}

#[test]
fn rejects_non_finite_direction_and_shape_overflow() {
    assert_eq!(
        GridGeometry::try_new(
            [1, 1],
            [Length::from_base(1.0), Length::from_base(1.0)],
            [Length::from_base(0.0), Length::from_base(0.0)],
            [[f64::INFINITY, 0.0], [0.0, 1.0]],
        ),
        Err(FieldError::NonFiniteDirection { row: 0, column: 0 })
    );
    assert_eq!(
        GridGeometry::try_new(
            [usize::MAX, 2],
            [Length::from_base(1.0), Length::from_base(1.0)],
            [Length::from_base(0.0), Length::from_base(0.0)],
            [[1.0, 0.0], [0.0, 1.0]],
        ),
        Err(FieldError::ShapeOverflow)
    );
}

#[test]
fn compatible_frames_require_exact_metadata_and_time() {
    let first_values = [Quantity::<f64, dimensions::Intensity>::from_base(1.0); 4];
    let second_values = [Quantity::<f64, dimensions::VolumetricPowerDensity>::from_base(1.0); 4];
    let first_time = Instant::new(aequitas::systems::si::quantities::Time::from_base(0.0))
        .expect("invariant: zero is finite");
    let second_time = Instant::new(aequitas::systems::si::quantities::Time::from_base(0.0))
        .expect("invariant: zero is finite");
    let first = FieldEnvelope::try_new(&first_values, geometry(), first_time)
        .expect("invariant: value count matches geometry");
    let second = FieldEnvelope::try_new(&second_values, geometry(), second_time)
        .expect("invariant: value count matches geometry");
    assert_eq!(first.validate_compatible(&second), Ok(()));

    let changed_geometry = GridGeometry::try_new(
        [2, 2],
        [Length::from_base(2.0), Length::from_base(1.0)],
        [Length::from_base(0.0), Length::from_base(0.0)],
        [[1.0, 0.0], [0.0, 1.0]],
    )
    .expect("invariant: changed spacing remains valid");
    let changed = FieldEnvelope::try_new(&second_values, changed_geometry, second_time)
        .expect("invariant: value count matches geometry");
    assert_eq!(
        first.validate_compatible(&changed),
        Err(FieldError::SpacingMismatch { axis: 0 })
    );

    let later = FieldEnvelope::try_new(
        &second_values,
        geometry(),
        Instant::new(aequitas::systems::si::quantities::Time::from_base(1.0))
            .expect("invariant: one is finite"),
    )
    .expect("invariant: value count matches geometry");
    assert_eq!(
        first.validate_compatible(&later),
        Err(FieldError::TimeMismatch)
    );
}

#[test]
fn rotated_frame_is_orthonormal_and_metadata_round_trips() {
    let direction = [[0.0_f64, -1.0], [1.0, 0.0]];
    let frame = GridGeometry::try_new(
        [2, 2],
        [Length::from_base(0.5), Length::from_base(0.75)],
        [Length::from_base(1.0), Length::from_base(-2.0)],
        direction,
    )
    .expect("invariant: quarter-turn direction is orthonormal");

    assert_eq!(frame.shape(), &[2, 2]);
    assert_eq!(frame.direction(), &direction);
    assert_eq!(frame.spacing()[0].into_base().to_bits(), 0.5_f64.to_bits());
    assert_eq!(
        frame.origin()[1].into_base().to_bits(),
        (-2.0_f64).to_bits()
    );
}

#[test]
fn compatible_frames_report_shape_origin_and_direction_mismatches() {
    let values = [Quantity::<f64, dimensions::Intensity>::from_base(1.0); 4];
    let time = Instant::new(aequitas::systems::si::quantities::Time::from_base(0.0))
        .expect("invariant: zero is finite");
    let first = FieldEnvelope::try_new(&values, geometry(), time)
        .expect("invariant: value count matches geometry");

    let shape = GridGeometry::try_new(
        [1, 4],
        [Length::from_base(1.0), Length::from_base(1.0)],
        [Length::from_base(0.0), Length::from_base(0.0)],
        [[1.0, 0.0], [0.0, 1.0]],
    )
    .expect("invariant: changed shape remains valid");
    let shape_field = FieldEnvelope::try_new(&values, shape, time)
        .expect("invariant: changed shape preserves value count");
    assert_eq!(
        first.validate_compatible(&shape_field),
        Err(FieldError::ShapeMismatch {
            axis: 0,
            expected: 2,
            actual: 1,
        })
    );

    let origin = GridGeometry::try_new(
        [2, 2],
        [Length::from_base(1.0), Length::from_base(1.0)],
        [Length::from_base(0.0), Length::from_base(1.0)],
        [[1.0, 0.0], [0.0, 1.0]],
    )
    .expect("invariant: changed origin remains valid");
    let origin_field = FieldEnvelope::try_new(&values, origin, time)
        .expect("invariant: changed origin preserves value count");
    assert_eq!(
        first.validate_compatible(&origin_field),
        Err(FieldError::OriginMismatch { axis: 1 })
    );

    let direction = GridGeometry::try_new(
        [2, 2],
        [Length::from_base(1.0), Length::from_base(1.0)],
        [Length::from_base(0.0), Length::from_base(0.0)],
        [[0.0, -1.0], [1.0, 0.0]],
    )
    .expect("invariant: rotated direction remains valid");
    let direction_field = FieldEnvelope::try_new(&values, direction, time)
        .expect("invariant: changed direction preserves value count");
    assert_eq!(
        first.validate_compatible(&direction_field),
        Err(FieldError::DirectionMismatch { row: 0, column: 0 })
    );
}

proptest! {
    #[test]
    fn positive_shape_product_is_preserved(first in 1_usize..8, second in 1_usize..8) {
        let shape = [first, second];
        let frame = GridGeometry::try_new(
            shape,
            [Length::from_base(1.0), Length::from_base(1.0)],
            [Length::from_base(0.0), Length::from_base(0.0)],
            [[1.0, 0.0], [0.0, 1.0]],
        ).expect("generated positive shape and identity frame are valid");
        prop_assert_eq!(frame.value_count(), first * second);
    }
}
