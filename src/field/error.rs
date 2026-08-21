//! Errors raised while constructing or comparing physical-field boundaries.

/// Failure while validating a typed physical-field exchange boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FieldError {
    /// The geometry rank is zero and therefore has no physical axes.
    ZeroRank,
    /// A grid extent is zero and cannot contain a field value.
    ZeroExtent {
        /// Axis whose extent is invalid.
        axis: usize,
    },
    /// The product of the grid extents does not fit in `usize`.
    ShapeOverflow,
    /// A grid spacing is non-finite or not strictly positive.
    InvalidSpacing {
        /// Axis whose spacing is invalid.
        axis: usize,
    },
    /// An origin coordinate is not finite.
    NonFiniteOrigin {
        /// Axis whose origin is invalid.
        axis: usize,
    },
    /// A direction-cosine entry is not finite.
    NonFiniteDirection {
        /// Row containing the invalid entry.
        row: usize,
        /// Column containing the invalid entry.
        column: usize,
    },
    /// The direction-cosine matrix is not orthonormal within its derived bound.
    NonOrthonormalDirection {
        /// Row of the failed dot-product check.
        row: usize,
        /// Column of the failed dot-product check.
        column: usize,
    },
    /// The value slice length does not match the grid cell count.
    ValueCount {
        /// Number of values required by the geometry.
        expected: usize,
        /// Number of values supplied by the caller.
        actual: usize,
    },
    /// Two geometries have different extents on one axis.
    ShapeMismatch {
        /// Axis whose extent differs.
        axis: usize,
        /// Extent in the first geometry.
        expected: usize,
        /// Extent in the second geometry.
        actual: usize,
    },
    /// Two geometries have different spacing on one axis.
    SpacingMismatch {
        /// Axis whose spacing differs.
        axis: usize,
    },
    /// Two geometries have different origins on one axis.
    OriginMismatch {
        /// Axis whose origin differs.
        axis: usize,
    },
    /// Two geometries have different direction cosines at one entry.
    DirectionMismatch {
        /// Row whose direction cosine differs.
        row: usize,
        /// Column whose direction cosine differs.
        column: usize,
    },
    /// Two fields are sampled at different simulation instants.
    TimeMismatch,
}
