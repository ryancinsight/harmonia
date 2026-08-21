//! Validated grid-frame metadata.

use aequitas::systems::si::quantities::Length;
use eunomia::RealField;

use super::FieldError;

// A dot product of RANK terms has at most RANK rounding sites. The factor also
// covers perturbations in the two input entries and the diagonal reference.
const ORTHONORMAL_TOLERANCE_FACTOR: f64 = 16.0;

/// Validated shape and coordinate frame for a Cartesian physical field.
///
/// `direction[row][column]` is the cosine of grid axis `row` with world axis
/// `column`. A grid displacement is mapped to world coordinates by multiplying
/// this matrix by the component-wise spacing-scaled index displacement. The
/// constructor validates every boundary value once, so consumers can exchange
/// borrowed values without repeating shape or frame checks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGeometry<T, const RANK: usize> {
    shape: [usize; RANK],
    spacing: [Length<T>; RANK],
    origin: [Length<T>; RANK],
    direction: [[T; RANK]; RANK],
    value_count: usize,
}

impl<T, const RANK: usize> GridGeometry<T, RANK>
where
    T: RealField,
{
    /// Validate and construct a grid geometry.
    ///
    /// Direction-cosine validation uses the bound
    /// `16 · RANK · ε_T`, where `ε_T` is [`RealField::EPSILON`]. The factor
    /// accounts for the two input roundings, one fused multiply-add per dot
    /// product term, and the diagonal reference rounding. This conservative
    /// bound is a construction-time validity threshold, not a numerical result
    /// tolerance.
    ///
    /// # Errors
    ///
    /// Returns a [`FieldError`] when the rank, extents, spacing, origin,
    /// direction matrix, or checked value-count product is invalid.
    pub fn try_new(
        shape: [usize; RANK],
        spacing: [Length<T>; RANK],
        origin: [Length<T>; RANK],
        direction: [[T; RANK]; RANK],
    ) -> Result<Self, FieldError> {
        if RANK == 0 {
            return Err(FieldError::ZeroRank);
        }

        let mut value_count = 1_usize;
        for (axis, &extent) in shape.iter().enumerate() {
            if extent == 0 {
                return Err(FieldError::ZeroExtent { axis });
            }
            value_count = value_count
                .checked_mul(extent)
                .ok_or(FieldError::ShapeOverflow)?;
        }

        for (axis, spacing_value) in spacing.iter().map(Length::as_base).enumerate() {
            let value = *spacing_value;
            if !value.is_finite() || value <= T::ZERO {
                return Err(FieldError::InvalidSpacing { axis });
            }
        }

        for (axis, origin_value) in origin.iter().map(Length::as_base).enumerate() {
            if !origin_value.is_finite() {
                return Err(FieldError::NonFiniteOrigin { axis });
            }
        }

        for (row, entries) in direction.iter().enumerate() {
            for (column, &value) in entries.iter().enumerate() {
                if !value.is_finite() {
                    return Err(FieldError::NonFiniteDirection { row, column });
                }
            }
        }

        // RANK is a structural parameter for a fixed-size matrix. The
        // conversion is used only to scale the construction-time error bound;
        // practical grid ranks are far below the precision-loss threshold.
        #[expect(
            clippy::cast_precision_loss,
            reason = "RANK scales the derived dot-product bound and is structural"
        )]
        let rank = RANK as f64;
        let tolerance = T::EPSILON * T::from_f64(ORTHONORMAL_TOLERANCE_FACTOR) * T::from_f64(rank);
        for (row, row_values) in direction.iter().enumerate() {
            for (column, column_values) in direction.iter().enumerate() {
                let dot = row_values
                    .iter()
                    .zip(column_values)
                    .fold(T::ZERO, |sum, (&left, &right)| {
                        left.scalar_fmadd(right, sum)
                    });
                let expected = if row == column { T::ONE } else { T::ZERO };
                if (dot - expected).abs() > tolerance {
                    return Err(FieldError::NonOrthonormalDirection { row, column });
                }
            }
        }

        Ok(Self {
            shape,
            spacing,
            origin,
            direction,
            value_count,
        })
    }

    /// Return the number of cells represented by the geometry.
    #[must_use]
    pub const fn value_count(&self) -> usize {
        self.value_count
    }

    /// Return the compile-time-rank grid extents.
    #[must_use]
    pub const fn shape(&self) -> &[usize; RANK] {
        &self.shape
    }

    /// Return the positive spacing for each grid axis.
    #[must_use]
    pub const fn spacing(&self) -> &[Length<T>; RANK] {
        &self.spacing
    }

    /// Return the finite world-space origin.
    #[must_use]
    pub const fn origin(&self) -> &[Length<T>; RANK] {
        &self.origin
    }

    /// Return the validated direction-cosine matrix.
    #[must_use]
    pub const fn direction(&self) -> &[[T; RANK]; RANK] {
        &self.direction
    }

    /// Validate that two fields use the same discrete frame.
    ///
    /// The comparison is exact because geometry metadata is a boundary
    /// contract: silently accepting a different frame would exchange values at
    /// the wrong physical locations. Numerical tolerances belong to the solver
    /// operating on the field values, not to this identity check.
    ///
    /// # Errors
    ///
    /// Returns the first mismatching frame component.
    pub fn validate_compatible(&self, other: &Self) -> Result<(), FieldError> {
        for (axis, (&expected, &actual)) in self.shape.iter().zip(&other.shape).enumerate() {
            if expected != actual {
                return Err(FieldError::ShapeMismatch {
                    axis,
                    expected,
                    actual,
                });
            }
        }
        for (axis, (expected, actual)) in self.spacing.iter().zip(&other.spacing).enumerate() {
            if expected != actual {
                return Err(FieldError::SpacingMismatch { axis });
            }
        }
        for (axis, (expected, actual)) in self.origin.iter().zip(&other.origin).enumerate() {
            if expected != actual {
                return Err(FieldError::OriginMismatch { axis });
            }
        }
        for (row, (expected_row, actual_row)) in
            self.direction.iter().zip(&other.direction).enumerate()
        {
            for (column, (&expected, &actual)) in expected_row.iter().zip(actual_row).enumerate() {
                if expected != actual {
                    return Err(FieldError::DirectionMismatch { row, column });
                }
            }
        }
        Ok(())
    }
}
