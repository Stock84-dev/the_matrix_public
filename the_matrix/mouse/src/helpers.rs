use std::collections::BTreeMap;
use std::fmt::Debug;
use std::ops::Range;
use std::path::Path;
use std::{mem, slice};

use num_traits::Num;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_value::Value;
use tokio::fs::{File, OpenOptions};

use crate::error::{Result, ResultCtxExt};
use crate::log::*;
use crate::num::traits::Float;

pub fn serialize_struct_to_map<S>(instance: &S) -> BTreeMap<Value, Value>
where
    S: Serialize,
{
    match serde_value::to_value(instance) {
        Ok(Value::Map(map)) => map,
        _ => panic!("expected a struct"),
    }
}

pub fn get_field_by_name<R>(serialized_struct: &mut BTreeMap<Value, Value>, field: &str) -> R
where
    R: DeserializeOwned,
{
    let key = Value::String(field.to_owned());

    let value = match serialized_struct.remove(&key) {
        Some(value) => value,
        None => panic!("no such field"),
    };

    match R::deserialize(value) {
        Ok(r) => r,
        Err(e) => panic!("{:?}", e),
    }
}

pub unsafe fn ptr_as_slice<'r, T, R>(ptr: *const T, len: usize) -> &'r [R] {
    let slice: &[R] = slice::from_raw_parts(ptr as *const _ as *const R, len);
    slice
}

pub unsafe fn ptr_as_slice_mut<'r, T, R>(ptr: *mut T, len: usize) -> &'r mut [R] {
    let slice: &mut [R] = slice::from_raw_parts_mut(ptr as *mut _ as *mut R, len);
    slice
}

pub unsafe fn object_as_slice<'r, T, R>(object: &T, len: usize) -> &'r [R] {
    ptr_as_slice(object as *const T, len)
}

pub unsafe fn object_as_slice_mut<'r, T, R>(object: &mut T, len: usize) -> &'r mut [R] {
    ptr_as_slice_mut(object as *mut T, len)
}

pub fn stdev<T: Float>(data: &[T]) -> T {
    let len = T::from(data.len()).unwrap();
    let avg = data.iter().fold(T::zero(), |acc: T, elem| acc + *elem) / len;
    let variance: T = data
        .iter()
        .fold(T::zero(), |acc: T, &d| acc + (d - avg) * (d - avg))
        / len;
    variance.sqrt()
}

pub fn forget_and_initialize<T>(dest: &mut T, mut src: T) {
    mem::swap(dest, &mut src);
    mem::forget(src);
}

pub async fn open_rwc_async(path: impl AsRef<Path>) -> Result<File> {
    Ok(OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .await?)
}

pub fn open_rwc_all(path: impl AsRef<Path>) -> Result<std::fs::File> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("Failed to open {}", path.display()))?)
}

pub fn open_ca_all(path: impl AsRef<Path>) -> Result<std::fs::File> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("Failed to open {}", path.display()))?)
}

pub fn range(start: usize, len: usize) -> Range<usize> {
    start..start + len
}

pub fn binary_search_max<R, I, F, E>(
    mut range_i: Range<I>,
    mut predicate: F,
    mut best: R,
    mut best_i: I,
) -> Result<(I, R), E>
where
    R: Num + Copy + PartialOrd,
    I: Num + Copy,
    F: FnMut(I) -> Result<R, E>,
{
    let two = I::one() + I::one();
    loop {
        let distance = range_i.end - range_i.start;
        if distance == I::zero() {
            break;
        }
        let middle = range_i.start + distance / two;
        let result = predicate(middle)?;
        if result > best {
            best = result;
            best_i = middle;
            range_i.start = middle;
        } else {
            range_i.end = middle;
        }
    }
    Ok((best_i, best))
}

pub fn log_error<F: FnOnce() -> Result<R, E>, R, E: Debug>(
    closure: F,
) -> impl FnOnce() -> Result<R, E> {
    || {
        let result = closure();
        if let Err(e) = &result {
            error!("{:?}", e);
        }
        result
    }
}

pub fn some_if<T>(condition: bool, mut constructor: impl FnMut() -> T) -> Option<T> {
    if condition {
        Some(constructor())
    } else {
        None
    }
}
