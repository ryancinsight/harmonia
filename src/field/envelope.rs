//! Borrowed, quantity-typed field values.

use aequitas::Quantity;
use eunomia::RealField;
use horae::time::Instant;

use super::{FieldError, GridGeometry};

/// Borrowed physical values with a compile-time quantity dimension and a
/// validated spatial frame.
///
/// The slice is borrowed exactly as supplied by the caller. Construction does
/// not allocate or copy values. The dimension parameter `D` is the Aequitas
/// quantity dimension, so an API accepting one physical quantity cannot receive
/// another quantity merely because both use the same scalar representation.
///
/// ```compile_fail
/// use aequitas::systems::si::{dimensions, quantities::Intensity};
/// use harmonia::{FieldEnvelope, GridGeometry};
/// use horae::time::Instant;
/// use aequitas::systems::si::quantities::{Length, Time};
///
/// fn accepts_power<'a>(
///     field: FieldEnvelope<'a, f64, dimensions::VolumetricPowerDensity, 1>,
/// ) {
///     let _ = field;
/// }
///
/// fn pass_intensity<'a>(values: &'a [Intensity<f64>]) {
///     let geometry = GridGeometry::try_new(
///         [1],
///         [Length::from_base(1.0)],
///         [Length::from_base(0.0)],
///         [[1.0]],
///     ).unwrap();
///     let time = Instant::new(Time::from_base(0.0)).unwrap();
///     let intensity = FieldEnvelope::try_new(values, geometry, time).unwrap();
///     accepts_power(intensity);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldEnvelope<'a, T, D, const RANK: usize> {
    values: &'a [Quantity<T, D>],
    geometry: GridGeometry<T, RANK>,
    time: Instant<T>,
}

impl<'a, T, D, const RANK: usize> FieldEnvelope<'a, T, D, RANK>
where
    T: RealField,
{
    /// Validate and construct a borrowed physical-field envelope.
    ///
    /// # Errors
    ///
    /// Returns [`FieldError::ValueCount`] when the borrowed slice does not
    /// contain exactly one value per geometry cell.
    pub fn try_new(
        values: &'a [Quantity<T, D>],
        geometry: GridGeometry<T, RANK>,
        time: Instant<T>,
    ) -> Result<Self, FieldError> {
        let actual = values.len();
        let expected = geometry.value_count();
        if actual != expected {
            return Err(FieldError::ValueCount { expected, actual });
        }
        Ok(Self {
            values,
            geometry,
            time,
        })
    }

    /// Return the borrowed quantity-typed values without allocation.
    #[must_use]
    pub const fn values(&self) -> &'a [Quantity<T, D>] {
        self.values
    }

    /// Return the validated spatial geometry.
    #[must_use]
    pub const fn geometry(&self) -> &GridGeometry<T, RANK> {
        &self.geometry
    }

    /// Return the simulation instant associated with the values.
    #[must_use]
    pub const fn time(&self) -> Instant<T>
    where
        T: Copy,
    {
        self.time
    }

    /// Validate frame and time compatibility with another quantity-typed field.
    ///
    /// `U` is independent of `D`, so this method can validate the spatial and
    /// temporal parts of an exchange while the field type itself continues to
    /// enforce the quantity dimension at compile time.
    ///
    /// # Errors
    ///
    /// Returns the first geometry or time mismatch.
    pub fn validate_compatible<U>(
        &self,
        other: &FieldEnvelope<'_, T, U, RANK>,
    ) -> Result<(), FieldError>
    where
        T: PartialEq,
    {
        self.geometry.validate_compatible(&other.geometry)?;
        if self.time != other.time {
            return Err(FieldError::TimeMismatch);
        }
        Ok(())
    }
}
