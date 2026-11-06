//! Rare underflow-boundary evaluation in native scalar precision.
//!
//! A product's low part can be too small for `T` even when it decides a final
//! subnormal tie. Keep each normalised mantissa in `T` and its binary exponent
//! separately, so no floating-point operation widens or underflows the tail.
//! This uses the locked provider's binary round-to-nearest-even, gradual
//! underflow, and fused `scalar_fmadd` semantics for its real scalar types.

use core::cmp::Ordering;

use eunomia::{NumericElement, RealField};

use super::fixed::two_sum;

// The update has five terms (one current value and two product pairs). A
// rounding threshold adds at most two more; TwoSum never increases the count.
const MAX_TERMS: usize = 7;

/// Discover the last nonzero binary power in the scalar's own precision.
///
/// The numeric reduction identity `MAX_VALUE` is infinity for these scalars,
/// so it cannot supply a finite format bound.
pub(super) fn minimum_subnormal_exponent<T: RealField>() -> Option<i32> {
    let one = <T as NumericElement>::ONE;
    let zero = <T as NumericElement>::ZERO;
    let mut lower = 0_i32;
    let mut upper = 1_i32;
    while one.scale_binary(-upper) != zero {
        lower = upper;
        upper = upper.checked_mul(2)?;
    }
    while upper - lower > 1 {
        let middle = lower + (upper - lower) / 2;
        if one.scale_binary(-middle) == zero {
            upper = middle;
        } else {
            lower = middle;
        }
    }
    Some(-lower)
}

#[derive(Clone, Copy)]
struct Term<T> {
    mantissa: T,
    exponent: i32,
}

#[derive(Clone, Copy)]
struct Terms<T> {
    values: [Term<T>; MAX_TERMS],
    len: usize,
}

impl<T: RealField> Terms<T> {
    fn new() -> Self {
        Self {
            values: [Term {
                mantissa: <T as NumericElement>::ZERO,
                exponent: 0,
            }; MAX_TERMS],
            len: 0,
        }
    }

    /// Insert a nonzero term as a mantissa of magnitude in `[1, 2)`.
    fn push(&mut self, value: T, exponent: i32) -> Option<()> {
        if value == <T as NumericElement>::ZERO {
            return Some(());
        }
        let shift = value.binary_exponent()?;
        let entry = self.values.get_mut(self.len)?;
        *entry = Term {
            mantissa: value.scale_binary(-shift),
            exponent: exponent.checked_add(shift)?,
        };
        self.len += 1;
        Some(())
    }

    /// Split a product at safe magnitude, before attaching its exponent.
    fn push_product(&mut self, left: T, right: T, negate: bool) -> Option<()> {
        if left == <T as NumericElement>::ZERO || right == <T as NumericElement>::ZERO {
            return Some(());
        }
        let left_exponent = left.binary_exponent()?;
        let right_exponent = right.binary_exponent()?;
        let left = left.scale_binary(-left_exponent);
        let right = right.scale_binary(-right_exponent);
        let high = left * right;
        let low = left.scalar_fmadd(right, -high);
        let exponent = left_exponent.checked_add(right_exponent)?;
        self.push(if negate { -high } else { high }, exponent)?;
        self.push(if negate { -low } else { low }, exponent)
    }

    fn remove(&mut self, index: usize) {
        self.len -= 1;
        self.values[index] = self.values[self.len];
    }

    /// Compare the exact sum with zero without dropping smaller signed tails.
    fn sign(mut self) -> Option<Ordering> {
        let zero = <T as NumericElement>::ZERO;
        loop {
            if self.len == 0 {
                return Some(Ordering::Equal);
            }
            let mut largest = 0;
            let mut next = 1;
            if self.len == 1 {
                return self.values[0].mantissa.partial_cmp(&zero);
            }
            if self.values[next].exponent > self.values[largest].exponent {
                core::mem::swap(&mut largest, &mut next);
            }
            for index in 2..self.len {
                if self.values[index].exponent > self.values[largest].exponent {
                    next = largest;
                    largest = index;
                } else if self.values[index].exponent > self.values[next].exponent {
                    next = index;
                }
            }

            let top = self.values[largest];
            let second = self.values[next];
            // With at most six other terms, a four-exponent gap makes their
            // combined magnitude strictly smaller than the leading term.
            if top.exponent - second.exponent >= 4 {
                return top.mantissa.partial_cmp(&zero);
            }
            let aligned = second.mantissa.scale_binary(second.exponent - top.exponent);
            let (high, low) = two_sum(top.mantissa, aligned);
            if largest < next {
                core::mem::swap(&mut largest, &mut next);
            }
            self.remove(largest);
            self.remove(next);
            self.push(high, top.exponent)?;
            self.push(low, top.exponent)?;
        }
    }

    /// Compare the magnitude of this signed sum with a two-term threshold.
    fn compare(
        self,
        negative: bool,
        threshold: Option<(T, i32)>,
        half_exponent: Option<i32>,
    ) -> Option<Ordering> {
        let mut difference = self;
        if negative {
            for term in &mut difference.values[..difference.len] {
                term.mantissa = -term.mantissa;
            }
        }
        if let Some((mantissa, exponent)) = threshold {
            difference.push(-mantissa, exponent)?;
        }
        if let Some(exponent) = half_exponent {
            difference.push(-<T as NumericElement>::ONE, exponent)?;
        }
        difference.sign()
    }
}

/// Round `current * (1 - weight) + candidate * weight` after cancellation.
///
/// The caller supplies finite endpoints and a finite weight in `(0, 1)`.
/// The exponent interval includes products of two minimum subnormals. Every
/// comparison and significand operation stays in `T`; exponent bookkeeping
/// and the fixed-size term array do not allocate.
pub(super) fn convex<T: RealField>(
    current: T,
    candidate: T,
    weight: T,
    minimum_subnormal: i32,
    precision: i32,
) -> Option<T> {
    let zero = <T as NumericElement>::ZERO;
    let one = <T as NumericElement>::ONE;
    let mut terms = Terms::new();
    terms.push(current, 0)?;
    terms.push_product(weight, candidate, false)?;
    terms.push_product(weight, current, true)?;
    let direction = terms.sign()?;
    if direction == Ordering::Equal {
        return Some(zero);
    }
    let negative = direction == Ordering::Less;
    let maximum_exponent = match (current.binary_exponent(), candidate.binary_exponent()) {
        (Some(first), Some(second)) => first.max(second),
        (Some(exponent), None) | (None, Some(exponent)) => exponent,
        (None, None) => return Some(zero),
    };

    // A nonzero sum is an integer multiple of the square of the minimum
    // subnormal. A convex result cannot exceed the largest finite endpoint.
    let mut lower = 2 * minimum_subnormal - 1;
    let mut upper = maximum_exponent + 1;
    while upper - lower > 1 {
        let middle = lower + (upper - lower) / 2;
        match terms.compare(negative, Some((one, middle)), None)? {
            Ordering::Less => upper = middle,
            Ordering::Equal | Ordering::Greater => lower = middle,
        }
    }
    let exponent = lower;
    if exponent < minimum_subnormal {
        let above_half =
            terms.compare(negative, Some((one, minimum_subnormal - 1)), None)? == Ordering::Greater;
        let rounded = if above_half {
            one.scale_binary(minimum_subnormal)
        } else {
            zero
        };
        return Some(if negative { -rounded } else { rounded });
    }

    let grid = minimum_subnormal.max(exponent - (precision - 1));
    let mut significand = one;
    let mut odd = true;
    let mut bit = exponent - 1;
    while bit >= grid {
        let trial = significand + one.scale_binary(bit - exponent);
        odd = terms.compare(negative, Some((trial, exponent)), None)? != Ordering::Less;
        if odd {
            significand = trial;
        }
        bit -= 1;
    }
    let halfway = terms.compare(negative, Some((significand, exponent)), Some(grid - 1))?;
    if halfway == Ordering::Greater || (halfway == Ordering::Equal && odd) {
        significand += one.scale_binary(grid - exponent);
    }
    let rounded = significand.scale_binary(exponent);
    Some(if negative { -rounded } else { rounded })
}
