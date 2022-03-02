pub use half::prelude::*;
pub use num_traits as traits;
use num_traits::{Float, Num};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
pub use rust_decimal::{
    Decimal, {self},
};
pub use rust_decimal_macros::*;

pub trait IntoDecimal {
    fn to_decimal(&self) -> Option<Decimal>;
}

impl IntoDecimal for f32 {
    fn to_decimal(&self) -> Option<Decimal> {
        Decimal::from_f32(*self)
    }
}

impl IntoDecimal for f64 {
    fn to_decimal(&self) -> Option<Decimal> {
        Decimal::from_f64(*self)
    }
}

pub trait FromMaybeDecimal {
    fn to_f32(&self) -> f32;
}

impl FromMaybeDecimal for Option<Decimal> {
    fn to_f32(&self) -> f32 {
        if let Some(decimal) = self {
            decimal.to_f32().unwrap()
        } else {
            f32::NAN
        }
    }
}
pub trait NumExt<T = Self> {
    fn max_mut(&mut self, value: T);
    fn min_mut(&mut self, value: T);
    fn average(&mut self, new_entry: T, new_count: T);
    /// NOTE: only works for integers. If it is divisible by specified number and has remainder then
    /// increases self to the number that is divisible by specified number.
    fn div_ceil(&self, denominator: T) -> Self;
    // returns true if number is within <base * (1 - rel), base * (1 + rel)>
    fn within_percent(&self, base: T, rel: T) -> bool;
}

impl<T: Num + PartialOrd + Copy> NumExt<T> for T {
    default fn max_mut(&mut self, value: T) {
        if *self < value {
            *self = value;
        }
    }

    default fn min_mut(&mut self, value: T) {
        if *self > value {
            *self = value;
        }
    }

    default fn average(&mut self, new_entry: T, new_count: T) {
        *self = *self * ((new_count - Self::one()) / new_count) + new_entry / new_count;
    }

    default fn div_ceil(&self, denominator: T) -> Self {
        (*self + denominator - T::one()) / denominator
    }

    fn within_percent(&self, base: T, rel: T) -> bool {
        *self > base * (T::one() - rel) && *self < base * (T::one() + rel)
    }
}

impl<T: Float> NumExt<T> for T {
    fn average(&mut self, new_entry: T, new_count: T) {
        *self = *self * ((new_count - Self::one()) / new_count) + new_entry / new_count;
        if self.is_nan() {
            *self = T::zero();
        }
    }
}
