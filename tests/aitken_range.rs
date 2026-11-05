//! Secant-factor range coverage through the public relaxation contract.

use eunomia::{NumericElement, RealField};
use harmonia::{AitkenRelaxation, Relaxation};

fn check_opposite_residuals<T: RealField>(magnitude: T, tolerance: T) {
    let zero = <T as NumericElement>::ZERO;
    let one = <T as NumericElement>::ONE;
    let half = T::from_f64(0.5);
    let mut policy = AitkenRelaxation::new(T::from_f64(0.25), one, tolerance)
        .expect("valid factor bounds and positive tolerance");
    let mut first = [zero];
    let mut second = [zero];
    policy
        .update_pair(&mut first, &[-magnitude], &mut second, &[magnitude])
        .expect("unit relaxation reaches each finite candidate");
    assert_eq!(first, [-magnitude]);
    assert_eq!(second, [magnitude]);

    // r_previous = +/-m, r = -/+m gives omega = 1/2. Every input and
    // output is finite even when the intermediate difference 2m overflows.
    policy
        .update_pair(&mut first, &[zero], &mut second, &[zero])
        .expect("the exact secant factor and both updated values are finite");
    assert_eq!(first, [-magnitude * half]);
    assert_eq!(second, [magnitude * half]);
}

#[test]
fn opposite_finite_residuals_preserve_the_half_factor() {
    check_opposite_residuals(f32::from_bits(254 << 23), f32::EPSILON);
    check_opposite_residuals(f64::from_bits(2046 << 52), f64::EPSILON);
}

#[test]
fn opposite_residuals_cover_every_binary_scale_and_finite_upper_boundary() {
    for bit in 0..23 {
        let magnitude = f32::from_bits(1 << bit);
        check_opposite_residuals(magnitude, magnitude);
    }
    for exponent in 1..=254 {
        let magnitude = f32::from_bits(exponent << 23);
        check_opposite_residuals(magnitude, magnitude);
    }
    for bit in 0..52 {
        let magnitude = f64::from_bits(1 << bit);
        check_opposite_residuals(magnitude, magnitude);
    }
    for exponent in 1..=2046 {
        let magnitude = f64::from_bits(exponent << 52);
        check_opposite_residuals(magnitude, magnitude);
    }
    check_opposite_residuals(f32::MAX, f32::MAX);
    check_opposite_residuals(f64::MAX, f64::MAX);
}

fn check_absolute_tolerance<T: RealField>(maximum: T) {
    let zero = <T as NumericElement>::ZERO;
    let one = <T as NumericElement>::ONE;
    let half = T::from_f64(0.5);
    let mut policy = AitkenRelaxation::new(half, one, maximum)
        .expect("the absolute tolerance may be the largest finite value");
    let mut current = [zero];
    let mut other = [];
    policy
        .update_pair(&mut current, &[-maximum * half], &mut other, &[])
        .expect("the first update uses unit relaxation");
    policy
        .update_pair(&mut current, &[zero], &mut other, &[])
        .expect("a difference equal to the tolerance retains the unit factor");
    assert_eq!(current, [zero]);
}

#[test]
fn finite_difference_at_the_absolute_threshold_reuses_history() {
    check_absolute_tolerance(f32::MAX);
    check_absolute_tolerance(f64::MAX);
}

fn check_large_product<T: RealField>(unit: T) {
    let three = T::from_f64(3.0);
    let four = T::from_f64(4.0);
    let mut second = [];
    // Set a component history using a window with the finite lower
    // bound eight; FMA keeps 8u - 4u finite although 8u alone overflows.
    let mut policy = AitkenRelaxation::new(T::from_f64(8.0), T::from_f64(8.0), T::EPSILON)
        .expect("a positive singleton factor interval is valid");
    let mut first = [-four * unit];
    policy
        .update_pair(&mut first, &[-three * unit], &mut second, &[])
        .expect("the fused update is finite");
    assert_eq!(first, [four * unit]);
    // The raw factor is four and the configured clamp is eight, so the next
    // interface is -4u. A spurious product infinity must not reject the step.
    policy
        .update_pair(&mut first, &[three * unit], &mut second, &[])
        .expect("the secant is finite before the configured clamp");
    assert_eq!(first, [-four * unit]);
}

#[test]
fn finite_secants_survive_an_overflowing_unscaled_product() {
    check_large_product(f32::from_bits(252 << 23));
    check_large_product(f64::from_bits(2044 << 52));
}
