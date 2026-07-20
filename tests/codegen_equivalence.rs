//! Concrete-reference fixture for static relaxation code generation.

use harmonia::{FullRelaxation, Relaxation};

#[inline(never)]
#[must_use]
/// Apply the concrete finite-candidate policy.
pub fn concrete_full_update(current: f64, candidate: f64) -> f64 {
    if candidate.is_finite() {
        candidate
    } else {
        current
    }
}

#[inline(never)]
#[must_use]
/// Apply the generic zero-sized full-relaxation policy.
pub fn generic_full_update(current: f64, candidate: f64) -> f64 {
    let mut destination = [current];
    match FullRelaxation.update(&mut destination, &[candidate]) {
        Ok(()) => destination[0],
        Err(_) => current,
    }
}

#[test]
fn generic_and_concrete_full_updates_have_identical_value_semantics() {
    for (current, candidate) in [
        (1.0_f64, 2.0_f64),
        (-3.0, 0.0),
        (4.0, f64::INFINITY),
        (5.0, f64::NAN),
    ] {
        assert_eq!(
            generic_full_update(current, candidate).to_bits(),
            concrete_full_update(current, candidate).to_bits()
        );
    }
}
