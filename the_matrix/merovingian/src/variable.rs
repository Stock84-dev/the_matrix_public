use std::f32::NAN;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::ops::{Deref, Index};
use std::slice::SliceIndex;

use bevy::prelude::*;
use graph::GraphPlugin;
use mouse::error::Result;
use serde::*;

#[derive(Component)]
pub struct VariablesNode {
    pub variables: Variables,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Variable {
    pub min: f32, // inclusive
    pub max: f32, // exclusive
    pub value: f32,
    pub stride: f32,
}

impl Variable {
    pub fn new(min: f32, max: f32, value: f32, stride: f32) -> Variable {
        Variable {
            min,
            max,
            value,
            stride,
        }
    }
    pub fn min(min: f32, max: f32, stride: f32) -> Variable {
        Variable {
            min,
            max,
            value: min,
            stride,
        }
    }
    // pub fn from_id(id: String) -> Variable {
    //     let min;
    //     let max;
    //     let stride;
    //     let value;
    //     scan!(id.bytes() => "{}:{}:{}:({})", min, max, stride, value);
    //     Variable {
    //         min,
    //         max,
    //         stride,
    //         value,
    //     }
    // }
    //
    // pub fn from_current_id(id: &str) -> Variable {
    //     let value;
    //     scan!(id.bytes() => "{}", value);
    //     Variable {
    //         min: NAN,
    //         max: NAN,
    //         stride: NAN,
    //         value,
    //     }
    // }
    pub fn clone(&self) -> Variable {
        Variable {
            min: self.min,
            max: self.max,
            stride: self.stride,
            value: self.value,
        }
    }

    pub fn next(&mut self) -> bool {
        self.value += self.stride;
        if self.value > self.max {
            // resetting
            self.value = self.min;
            return false;
        }
        true
    }

    // pub fn get_id(&self) -> String {
    //     // "0.1:99.1:0.01:(49.03)"
    //     format!("{}:{}:{}:({})", self.min, self.max, self.stride, self.value)
    // }

    pub fn get_n_possible_combinations(&self) -> u64 {
        // rounding to avoid floating point precision issues
        ((self.max - self.min) / self.stride).round() as u64
    }

    pub fn get_current_id(&self) -> String {
        self.value.to_string()
    }

    pub fn digit(&self) -> u32 {
        // rounding to avoid floating point precision issues
        ((self.value - self.min) / self.stride).round() as u32
    }

    pub fn digit_with(&self, value: f32) -> u32 {
        // rounding to avoid floating point precision issues
        ((value - self.min) / self.stride).round() as u32
    }

    pub fn set_digit(&mut self, digit: u32) {
        self.value = self.min + self.stride * digit as f32;
    }

    pub fn base(&self) -> u32 {
        // rounding to avoid floating point precision issues
        ((self.max - self.min) / self.stride).round() as u32
    }

    /// Returns true if min, max, and stride are the same.
    pub fn compare_bounds(&self, variable: &Variable) -> bool {
        self.min == variable.min && self.max == variable.max && self.stride == variable.stride
    }

    // Sets value to min.
    pub fn reset(&mut self) {
        self.value = self.min;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Variables {
    #[serde(rename = "$value")]
    variables: Vec<Variable>,
    //    #[serde(skip_serializing)]
    max_combinations: u64,
    //    #[serde(skip_serializing)]
    current_combination: u64,
}

impl Variables {
    pub fn new(variables: Vec<Variable>) -> Variables {
        Variables {
            max_combinations: max_combinations(&variables),
            current_combination: current_combination(&variables),
            variables,
        }
    }

    // NOTE: There is exactly the same function in our opencl codebase.
    /// Increases variable by it's stride, if value is bigger than max then its set to min and
    /// increases next variable. Treats variables as a number consisting of digits with different
    /// bases. Returns index of most significant variable that is changed. Most significant
    /// is at index of 0. Returns -1 if overflows.
    pub fn increase(&mut self, n_times: u64) -> Option<usize> {
        let result = increase(&mut self.variables, n_times);
        if result != None {
            self.current_combination = current_combination(&self.variables);
        } else {
            self.current_combination = self.max_combinations;
        }
        result
    }

    pub fn set_combination(&mut self, target: u64) {
        if self.current_combination > target {
            self.reset();
        }
        increase_to_combination(&mut self.variables, target);
        self.current_combination = target;
    }

    pub fn increase_range(
        &mut self,
        start: usize,
        end_excluding: usize,
        n_times: u64,
    ) -> Option<usize> {
        let result = increase(&mut self.variables[start..end_excluding], n_times);
        if let Some(ref _i) = result {
            self.current_combination += n_times as u64;
        }
        result
    }

    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    pub fn combinations_until_overflow(&self) -> u64 {
        self.max_combinations - self.current_combination
    }

    pub fn into_inner(self) -> Vec<Variable> {
        self.variables
    }

    pub fn current_combination(&self) -> u64 {
        self.current_combination
    }

    pub fn max_combinations(&self) -> u64 {
        self.max_combinations
    }

    pub fn max_combinations_range(&self, from_id: usize, to_id_excluding: usize) -> u64 {
        max_combinations(&self.variables[from_id..to_id_excluding])
    }

    pub fn current_combination_range(&self, from_id: usize, to_id_excluding: usize) -> u64 {
        current_combination(&self.variables[from_id..to_id_excluding])
    }

    /// Returns number of combinations if variables gets increased by 'n_times' from a specific
    /// id;
    pub fn current_combination_if_increased(
        &self,
        from_id: usize,
        to_id: usize,
        n_times: u64,
    ) -> u64 {
        let mut vars = self.variables.clone();
        // We can ignore return of this function because variables are already increased to max if
        // it overflows.
        let _b = increase(&mut vars[from_id..to_id], n_times);
        increase_to_max_base(&mut vars[to_id..]);
        current_combination(&vars)
    }

    /// Returns true if min, max, and stride are the same for all variables.
    pub fn compare_bounds(&self, variables: &Variables) -> bool {
        compare_bounds(self.variables(), variables.variables())
    }

    pub fn set_variables(&mut self, variables: Vec<Variable>) {
        self.variables = variables;
        self.current_combination = current_combination(self.variables());
        self.max_combinations = max_combinations(self.variables());
    }

    pub fn reset(&mut self) {
        reset(&mut self.variables);
        self.current_combination = 0;
    }
}

fn increase_to_max_base(variables: &mut [Variable]) {
    for i in 0..variables.len() {
        loop {
            if variables[i].value + variables[i].stride >= variables[i].max {
                break;
            }
            variables[i].value += variables[i].stride;
        }
    }
}

/// Returns position of a most significant digit that has changed.
/// Returns `None` if overflow happens.
pub fn increase(variables: &mut [Variable], n_times: u64) -> Option<usize> {
    let mut carry = n_times;
    for i in (0..=(variables.len() - 1)).rev() {
        let base = variables[i].base() as u64;
        let value = variables[i].digit() as u64 + carry;
        let tmp_value = value % base;
        carry = value / base;
        variables[i].value = variables[i].min + variables[i].stride * tmp_value as f32;
        if carry == 0 {
            return Some(i);
        }
    }
    None
}

pub fn set_combination(variables: &mut [Variable], target: u64) {
    reset(variables);
    increase_to_combination(variables, target);
}

pub fn increase_to_combination(variables: &mut [Variable], target: u64) {
    let current_comb = current_combination(&variables);
    if cfg!(debug_assertions) {
        let max_combination = max_combinations(variables);
        if target > max_combination {
            panic!("Target bigger than max number of combinations.");
        } else if target < current_comb {
            panic!("Target is less than current combination.");
        }
    }
    increase(variables, target - current_comb);
}

impl<T: SliceIndex<[Variable]>> Index<T> for Variables {
    type Output = T::Output;

    fn index(&self, index: T) -> &Self::Output {
        &self.variables[index]
    }
}

impl Deref for Variables {
    type Target = Vec<Variable>;

    fn deref(&self) -> &Self::Target {
        &self.variables
    }
}

pub fn max_combinations(variables: &[Variable]) -> u64 {
    let mut combinations = 1;
    for i in 0..variables.len() {
        combinations *= variables[i].base() as u64;
    }

    combinations
}

pub fn current_combination(variables: &[Variable]) -> u64 {
    let mut bases = max_combinations(&variables[1..]);
    let mut prev_value = variables[0].digit() as u64;
    let mut values = 1;
    for i in 1..variables.len() {
        values += prev_value * bases;
        let base = variables[i].base() as u64;
        if base == 0 {
            debug!("{:#?}", variables[i]);
        }
        bases /= base;
        prev_value = variables[i].digit() as u64;
    }
    values += prev_value;

    values - 1
}

/// Returns true if min, max, and stride are the same for all variables.
pub fn compare_bounds(left: &[Variable], right: &[Variable]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    for i in 0..left.len() {
        if !left[i].compare_bounds(&right[i]) {
            return false;
        }
    }
    return true;
}

/// Sets all values to min.
pub fn reset(variables: &mut [Variable]) {
    for var in variables {
        var.reset();
    }
}

/// returns true if all variables are set to min
pub fn all_min(variables: &[Variable]) -> bool {
    variables.iter().all(|x| x.value == x.min)
}

pub fn from_values(values: &Vec<f32>) -> Vec<Variable> {
    values
        .iter()
        .map(|x| Variable {
            min: NAN,
            max: NAN,
            value: *x,
            stride: NAN,
        })
        .collect()
}

/// Sorts an array of values by variable in a way that provided variable becomes most significant.
/// While sorting it writes to provided writer.
/// The caller must ensure that array length is equal to max combinations of variables also
/// the array must already be sorted by variables. For minimal example check comment inside this
/// function.
pub fn sort_array<T, R: Read + Seek, W: Write>(
    variables: &Vec<Variable>,
    by: usize,
    reader: &mut BufReader<R>,
    writer: &mut W,
) -> Result<()> {
    // Provided array sorted with these variables with these bases.
    // 2 3 4 5
    // var 0 000000000000000000000000111111111111111111111111222222222222222222222222333333333333333333333333444444444444444444444444
    // var 1 000000111111222222333333000000111111222222333333000000111111222222333333000000111111222222333333000000111111222222333333
    // var 2 001122001122001122001122001122001122001122001122001122001122001122001122001122001122001122001122001122001122001122001122
    // var 3 010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101
    //
    // Sorted array by variable at index 3
    // var 3 000000000000000000000000000000000000000000000000000000000000111111111111111111111111111111111111111111111111111111111111
    // var 0 000000000000111111111111222222222222333333333333444444444444000000000000111111111111222222222222333333333333444444444444
    // var 1 000111222333000111222333000111222333000111222333000111222333000111222333000111222333000111222333000111222333000111222333
    // var 2 012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012012
    //
    // Sorted array by variable at index 2
    // var 2 000000000000000000000000000000000000000011111111111111111111111111111111111111112222222222222222222222222222222222222222
    // var 0 000000001111111122222222333333334444444400000000111111112222222233333333444444440000000011111111222222223333333344444444
    // var 1 001122330011223300112233001122330011223300112233001122330011223300112233001122330011223300112233001122330011223300112233
    // var 3 010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101
    let file_size = reader.seek(SeekFrom::End(0))?;
    let mut variables = variables.clone();
    reset(&mut variables[..]);
    let unit_len = if by + 1 == variables.len() {
        1
    } else {
        max_combinations(&variables[by + 1..])
    };
    let element_size = std::mem::size_of::<T>() as u64;
    let base = variables[by].base() as u64;
    let stride = unit_len * base * element_size;
    let mut i;
    let mut buf = vec![0u8; unit_len as usize * element_size as usize];
    for offset in 0..base {
        i = offset * unit_len * element_size;
        reader.seek(SeekFrom::Start(i))?;
        debug!("{:#?}", i);
        reader.read_exact(&mut buf)?;
        writer.write_all(&buf)?;
        i += stride;
        while i < file_size {
            reader.seek_relative(stride as i64 - buf.len() as i64)?;
            reader.read_exact(&mut buf)?;
            writer.write_all(&buf)?;
            i += stride;
        }
    }
    Ok(())
}

/// returns last index where variables are different
pub fn diff(a: &[Variable], b: &[Variable]) -> usize {
    for i in (0..a.len()).rev() {
        if a[i].value != b[i].value {
            return i;
        }
    }
    0
}

pub fn set_max_combination_to_current(variables: &mut [Variable]) {
    for v in variables {
        v.max = v.value + v.stride;
    }
}

#[cfg(test)]
mod t_variable {
    use test_helper::*;

    use super::*;
    #[test]
    fn t_increase() {
        configure_logging_once();
        let mut variables = vec![
            Variable {
                min: 60.0,
                max: 300.0,
                value: 60.0,
                stride: 60.0,
            },
            Variable {
                min: 0.0,
                max: 5.0,
                value: 0.0,
                stride: 1.0,
            },
            Variable {
                min: 14.,
                max: 20.,
                value: 14.,
                stride: 1.,
            },
            Variable {
                min: 0.,
                max: 100.,
                value: 0.,
                stride: 1.,
            },
            Variable {
                min: 0.,
                max: 100.,
                value: 0.,
                stride: 1.,
            },
        ];
        increase(&mut variables, 0);
        a_eq!(current_combination(&variables), 1);
        increase_to_combination(&mut variables, 1197344);
        increase(&mut variables, 2656);
        a_eq!(current_combination(&variables), 1197344 + 2656);
        increase(&mut variables, 0);
        a_eq!(current_combination(&variables), 1197344 + 2656);
    }

    #[test]
    fn t_increase_to_combination() {
        configure_logging_once();
        let mut variables = vec![
            Variable {
                min: 86400.,
                max: 86400. * 5.,
                value: 86400.,
                stride: 86400.,
            },
            Variable {
                min: 0.0,
                max: 5.0,
                value: 0.0,
                stride: 1.0,
            },
            Variable {
                min: 14.,
                max: 20.,
                value: 14.,
                stride: 1.,
            },
            Variable {
                min: 0.,
                max: 100.,
                value: 81.,
                stride: 1.,
            },
            Variable {
                min: 0.,
                max: 100.,
                value: 92.,
                stride: 1.,
            },
        ];
        increase_to_combination(&mut variables, 10_234);
        a_eq!(variables[4].value, 33.);
        a_eq!(variables[3].value, 2.);
        a_eq!(variables[2].value, 15.);
        a_eq!(variables[1].value, 0.);
        a_eq!(variables[0].value, 86400.);
        increase_to_combination(&mut variables, 300_000);
        a_eq!(variables[4].value, 99.);
        a_eq!(variables[3].value, 99.);
        a_eq!(variables[2].value, 19.);
        a_eq!(variables[1].value, 4.);
        a_eq!(variables[0].value, 86400.);
        increase_to_combination(&mut variables, 300_001);
        a_eq!(variables[4].value, 0.);
        a_eq!(variables[3].value, 0.);
        a_eq!(variables[2].value, 14.);
        a_eq!(variables[1].value, 0.);
        a_eq!(variables[0].value, 86400. * 2.);
    }

    #[test]
    fn t_current_combination() {
        configure_logging_once();
        let mut variables = vec![
            Variable {
                min: 86400.,
                max: 86400. * 5.,
                value: 86400.,
                stride: 86400.,
            },
            Variable {
                min: 0.0,
                max: 5.0,
                value: 0.0,
                stride: 1.0,
            },
            Variable {
                min: 14.,
                max: 20.,
                value: 14.,
                stride: 1.,
            },
        ];
        a_eq!(current_combination(&variables), 1);
        increase(&mut variables, 13);
        a_eq!(current_combination(&variables), 14);
        increase_to_max_base(&mut variables);
        a_eq!(current_combination(&variables), 6 * 5 * 4);
    }
}
