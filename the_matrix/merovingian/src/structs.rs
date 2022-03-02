use std::ops::{Bound, RangeBounds};

use mouse::{Pod, Zeroable};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BatchHeader {
    pub start_combination: u64,
    pub end_combination_inclusive: u64,
    pub block_size: u64,
    pub n_combinations: u32,
    pub ranges: Vec<RangeInclusive<f32>>,
}

#[repr(C)]
#[derive(new, Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RangeInclusive<T> {
    pub start: T,
    pub end: T,
}

// SAFETY: Pod can be implemented for generic structs that only contain generic fields of same type
// https://github.com/Lokathor/bytemuck/issues/75
unsafe impl<T: Pod> Pod for RangeInclusive<T> {}
unsafe impl<T: Zeroable> Zeroable for RangeInclusive<T> {}

impl<T: PartialOrd> RangeInclusive<T> {
    pub fn contains(&self, item: &T) -> bool {
        item >= &self.start && item <= &self.end
    }
}

impl<T> RangeBounds<T> for RangeInclusive<&T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(self.start)
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(self.end)
    }
}
