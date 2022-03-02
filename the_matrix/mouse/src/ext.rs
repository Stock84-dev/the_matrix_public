use std::any::TypeId;
use std::borrow::Borrow;
use std::future::Future;
use std::io::{Seek, SeekFrom};
use std::mem;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::Path;
use std::pin::Pin;
use std::str::pattern::{Pattern, ReverseSearcher};

use anyhow::Error;
use bytemuck::Pod;
use futures_util::{Stream, StreamExt};
use num_traits::{Num, One, Zero};
use tokio::pin;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::helpers::{ptr_as_slice, ptr_as_slice_mut};
use crate::macros::tokio::io::AsyncSeekExt;
use crate::prelude::*;
use crate::{error, helpers};

pub trait VecExt<T> {
    /// Keeps only the elements specified by the predicate.
    /// Returns true if all elements are still there.
    fn keep<F>(&mut self, f: F) -> bool
    where
        F: FnMut(&mut T) -> bool;
    /// Keeps only the elements specified by the predicate.
    /// Returns true if all elements are still there.
    fn keep_enumerated<F>(&mut self, f: F) -> bool
    where
        F: FnMut((usize, &mut T)) -> bool;
    /// Keeps only the elements specified by the predicate.
    /// Returns true if all elements are still there.
    fn keep_with_offset<F>(&mut self, f: F, start_id: usize) -> bool
    where
        F: FnMut(&mut T) -> bool;
    /// Returns size of vector contents in bytes
    fn size(&self) -> usize;
}

impl<T: Send> VecExt<T> for Vec<T> {
    fn keep<F>(&mut self, f: F) -> bool
    where
        F: FnMut(&mut T) -> bool,
    {
        self.keep_with_offset(f, 0)
    }

    fn keep_enumerated<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut((usize, &mut T)) -> bool,
    {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut **self;

            for i in 0..len {
                if !f((i, &mut v[i])) {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }

        if del > 0 {
            self.truncate(len - del);
        }
        !(del > 0)
    }

    fn keep_with_offset<F>(&mut self, mut f: F, start_id: usize) -> bool
    where
        F: FnMut(&mut T) -> bool,
    {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut **self;

            for i in start_id..len {
                if !f(&mut v[i]) {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }

        if del > 0 {
            self.truncate(len - del);
        }
        !(del > 0)
    }

    fn size(&self) -> usize {
        self.len() * T::size()
    }
}

pub trait VecBytemuckExt {
    fn alloc_set_len(&mut self, len: usize);
}

impl<T: Pod> VecBytemuckExt for Vec<T> {
    fn alloc_set_len(&mut self, len: usize) {
        if len > self.len() {
            self.reserve_exact(len - self.len());
        }
        unsafe {
            self.set_len(len);
        }
    }
}

pub trait FileExt<T = Self> {
    /// Returns the size of a file. Does not change file pointer.
    fn size(&mut self) -> error::Result<u64>;
}

impl<T: Seek> FileExt<T> for T {
    fn size(&mut self) -> error::Result<u64> {
        let current_pos = self.seek(SeekFrom::Current(0))?;
        let size = self.seek(SeekFrom::End(0))?;
        self.seek(SeekFrom::Start(current_pos))?;
        return Ok(size);
    }
}

#[async_trait]
pub trait AsyncFileExt<T = Self> {
    /// Returns the size of a file. Does not change file pointer.
    async fn size(&mut self) -> error::Result<u64>;
}

#[async_trait]
impl<T: AsyncSeekExt + Unpin + Send> AsyncFileExt<T> for T {
    async fn size(&mut self) -> error::Result<u64> {
        let current_pos = self.seek(SeekFrom::Current(0)).await?;
        let size = self.seek(SeekFrom::End(0)).await?;
        self.seek(SeekFrom::Start(current_pos)).await?;
        return Ok(size);
    }
}

#[async_trait]
pub trait PathExt {
    async fn exists_async(&self) -> bool;
    async fn is_dir_async(&self) -> std::io::Result<bool>;
}

#[async_trait]
impl<T: AsRef<Path> + Send + Sync> PathExt for T {
    async fn exists_async(&self) -> bool {
        tokio::fs::metadata(self).await.is_ok()
    }

    async fn is_dir_async(&self) -> std::io::Result<bool> {
        Ok(tokio::fs::metadata(self).await?.is_dir())
    }
}

pub trait StrExt {
    /// Returns a position of a first character specified by line index;
    fn find_line(&self, line_id: usize) -> Option<usize>;
    /// Returns the byte index of the first character of this string slice that matches nth pattern.
    fn findn<'a, P: Pattern<'a> + Clone>(&'a self, pat: P, n: usize) -> Option<usize>;
    /// Returns the byte index of the first character of this string slice that matches nth pattern,
    /// searches backwards.
    fn rfindn<'a, P>(&'a self, pat: P, n: usize) -> Option<usize>
    where
        P: Pattern<'a> + Clone,
        P::Searcher: ReverseSearcher<'a>;
}

impl StrExt for str {
    fn find_line(&self, line_id: usize) -> Option<usize> {
        let mut i = 0;
        let mut line = 0;
        while line < line_id {
            i += self[i..].find('\n')? + 1;
            line += 1;
        }
        Some(i)
    }

    fn findn<'a, P: Pattern<'a> + Clone>(&'a self, pat: P, n: usize) -> Option<usize> {
        let mut i = 0;
        let mut current_n = 0;
        while current_n < n || i >= self.len() {
            i += self[i..].find(pat.clone())? + 1;
            current_n += 1;
        }
        Some(i)
    }

    fn rfindn<'a, P>(&'a self, pat: P, n: usize) -> Option<usize>
    where
        P: Pattern<'a> + Clone,
        P::Searcher: ReverseSearcher<'a>,
    {
        let mut i = self.len();
        let mut current_n = 0;
        while current_n < n || i <= 0 {
            i -= self[..i].rfind(pat.clone())? + 1;
            current_n += 1;
        }
        Some(i)
    }
}

pub trait StringExt {
    fn surround(&mut self, value: char);
    fn surround_str(&mut self, value: &str);
    fn replace_char(&mut self, i: usize, c: char);
}

impl StringExt for String {
    fn surround(&mut self, value: char) {
        self.insert(0, value);
        self.push(value);
    }

    fn surround_str(&mut self, value: &str) {
        self.insert_str(0, value);
        self.push_str(value);
    }

    fn replace_char(&mut self, i: usize, c: char) {
        let mut tmp = [0u8; 4];
        let s = c.encode_utf8(&mut tmp);
        self.replace_range(i..i + 1, s);
    }
}

pub trait Transmutations: Sized {
    fn as_one_slice<'a>(&'a self) -> &'a [Self];
    fn as_one_slice_mut<'a>(&'a mut self) -> &'a mut [Self];
    fn as_u8_slice<'a>(&'a self) -> &'a [u8];
    unsafe fn as_u8_slice_mut<'a>(&'a mut self) -> &'a mut [u8];
    unsafe fn as_static<'a>(&'a self) -> &'static Self;
    unsafe fn as_static_mut<'a>(&'a mut self) -> &'static mut Self;
    unsafe fn as_mut_cast<'a>(&'a self) -> &'a mut Self;
    unsafe fn from_u8_slice<'a>(slice: &'a [u8]) -> &'a Self;
    unsafe fn from_u8_slice_mut<'a>(slice: &'a mut [u8]) -> &'a mut Self;
}

impl<T: Sized> Transmutations for T {
    fn as_one_slice<'a>(&'a self) -> &'a [Self] {
        unsafe { helpers::ptr_as_slice(self as *const Self, 1) }
    }

    fn as_one_slice_mut<'a>(&'a mut self) -> &'a mut [Self] {
        unsafe { helpers::ptr_as_slice_mut(self as *mut Self, 1) }
    }

    fn as_u8_slice<'a>(&'a self) -> &'a [u8] {
        unsafe { helpers::ptr_as_slice(self as *const Self, T::size()) }
    }

    unsafe fn as_u8_slice_mut<'a>(&'a mut self) -> &'a mut [u8] {
        helpers::ptr_as_slice_mut(self as *mut Self, T::size())
    }

    unsafe fn as_static<'a>(&'a self) -> &'static Self {
        std::mem::transmute(self)
    }

    unsafe fn as_static_mut<'a>(&'a mut self) -> &'static mut Self {
        std::mem::transmute(self)
    }

    unsafe fn as_mut_cast<'a>(&'a self) -> &'a mut Self {
        &mut *(self as *const _ as *mut Self)
    }

    unsafe fn from_u8_slice<'a>(slice: &'a [u8]) -> &'a Self {
        &*(slice.as_ptr() as *const Self)
    }

    unsafe fn from_u8_slice_mut<'a>(slice: &'a mut [u8]) -> &'a mut Self {
        &mut *(slice.as_mut_ptr() as *mut Self)
    }
}

pub trait UninitializedCollection {
    unsafe fn uninitialized_collection(len: usize) -> Self;
}

impl<T> UninitializedCollection for Vec<T> {
    unsafe fn uninitialized_collection(len: usize) -> Self {
        let mut vec = Vec::with_capacity(len);
        vec.set_len(len);
        vec
    }
}

pub trait Uninitialized {
    /// Creates an object without initializing its memory.
    /// Safety: An object must be forgotten immediately
    unsafe fn uninitialized_unsafe() -> Self;
    // fn forget(self);
}

impl<T> Uninitialized for T {
    unsafe fn uninitialized_unsafe() -> Self {
        MaybeUninit::uninit().assume_init()
    }
    // fn forget(self) {
    //     std::mem::forget(self);
    // }
}

pub trait Initialized: Sized {
    /// Forgets original and initializes with other value.
    fn initialize(&mut self, mut value: Self) {
        mem::swap(self, &mut value);
        mem::forget(value);
    }
}

impl<T> Initialized for T {}

pub trait AsPinned {
    unsafe fn as_pin_mut_unchecked(&mut self) -> Pin<&mut Self> {
        Pin::new_unchecked(&mut *(self as *mut _))
    }
    fn as_pin_mut(&mut self) -> Pin<&mut Self>
    where
        Self: Unpin,
    {
        Pin::new(self)
    }
}

impl<T> AsPinned for T {}

pub trait IterExt: Iterator + Sized + 'static {
    fn boxed(self) -> Box<dyn Iterator<Item = Self::Item>> {
        Box::new(self)
    }
}

impl<T: Iterator + 'static> IterExt for T {}

pub trait IterNumExt<I, B>: Iterator<Item = B> + Sized
where
    B: Borrow<I>,
    I: Num + Zero + One + Clone,
{
    fn average(self) -> I {
        let mut sum = I::zero();
        let mut count = I::one();
        for item in self {
            count = count + I::one();
            sum = sum + item.borrow().clone();
        }
        sum / count
    }
}

// impl<T: Iterator<Item: Num + Zero + One> + 'static> IterNumExt for T {}
impl<T, I, B> IterNumExt<I, B> for T
where
    T: Iterator<Item = B>,
    B: Borrow<I>,
    I: Num + Zero + One + Clone,
{
}

pub trait SizeOfVal {
    /// Returns the size of the pointed-to value in bytes.
    ///
    /// This is usually the same as `size_of::<T>()`. However, when `T` *has* no
    /// statically-known size, e.g., a slice [`[T]`][slice] or a [trait object],
    /// then `size_of_val` can be used to get the dynamically-known size.
    fn size_of_val(&self) -> usize;
}

impl<T: Sized> const SizeOfVal for T {
    fn size_of_val(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

pub trait StaticSize {
    /// Returns the size of a type in bytes.
    fn size() -> usize;
}

impl<T: Sized> const StaticSize for T {
    fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

pub trait OptionExt {
    type Nullable;
    fn on_mut<'a>(&'a mut self, callback: impl FnMut(&'a mut Self::Nullable));
    fn on_ref<'a>(&'a self, callback: impl FnMut(&'a Self::Nullable));
    fn ok(self) -> Result<Self::Nullable, crate::error::Error>;
}

pub trait OptionExtResult<'a, O: Default, E> {
    type Nullable: 'a;
    fn on_mut_result(
        &'a mut self,
        callback: impl FnMut(&'a mut Self::Nullable) -> Result<O, E>,
    ) -> Result<O, E>;
    fn on_ref_result(
        &'a self,
        callback: impl FnMut(&'a Self::Nullable) -> Result<O, E>,
    ) -> Result<O, E>;
}

impl<T> OptionExt for Option<T> {
    type Nullable = T;

    fn on_mut<'a>(&'a mut self, callback: impl FnMut(&'a mut Self::Nullable)) {
        self.iter_mut().for_each(callback);
    }

    fn on_ref<'a>(&'a self, callback: impl FnMut(&'a Self::Nullable)) {
        self.iter().for_each(callback);
    }

    fn ok(self) -> Result<Self::Nullable, Error> {
        self.ok_or(anyhow!("called `Option::unwrap()` on a `None` value"))
    }
}
impl<'a, T: 'a, O: Default, E> OptionExtResult<'a, O, E> for Option<T> {
    type Nullable = T;

    fn on_mut_result(
        &'a mut self,
        mut callback: impl FnMut(&'a mut Self::Nullable) -> Result<O, E>,
    ) -> Result<O, E> {
        if let Some(nullable) = self.as_mut() {
            callback(nullable)
        } else {
            Ok(Default::default())
        }
    }

    fn on_ref_result(
        &'a self,
        mut callback: impl FnMut(&'a Self::Nullable) -> Result<O, E>,
    ) -> Result<O, E> {
        if let Some(nullable) = self.as_ref() {
            callback(nullable)
        } else {
            Ok(Default::default())
        }
    }
}

pub trait SliceExt {
    unsafe fn transmute_slice<'a, T>(&self) -> &'a [T];
    unsafe fn transmute_slice_mut<'a, T>(&mut self) -> &'a mut [T];
}

impl<U> SliceExt for [U] {
    unsafe fn transmute_slice<'a, T>(&self) -> &'a [T] {
        let size = self.len() * U::size();
        debug_assert!(size % T::size() == 0);
        ptr_as_slice(self.as_ptr(), size / T::size())
    }

    unsafe fn transmute_slice_mut<'a, T>(&mut self) -> &'a mut [T] {
        let size = self.len() * U::size();
        debug_assert!(size % T::size() == 0);
        ptr_as_slice_mut(self.as_mut_ptr(), size / T::size())
    }
}

lazy_static::lazy_static! {
    static ref TOKIO_HANDLE: Box<Handle> = Box::new(Handle::current());
}

pub async fn set_tokio_handle() {
    TOKIO_HANDLE.spawn(async {});
}

pub trait ExecuteFutureExt: Future
where
    Self: Sized,
{
    fn block(self) -> Self::Output {
        TOKIO_HANDLE.block_on(self)
    }

    fn spawn(self) -> JoinHandle<Self::Output>
    where
        Self: Send + 'static,
        Self::Output: Send,
    {
        TOKIO_HANDLE.spawn(self)
    }
}

impl<F: Future> ExecuteFutureExt for F {}

pub trait PodExt: Pod {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>())
        }
    }

    fn uninitialized() -> Self {
        unsafe { MaybeUninit::uninit().assume_init() }
    }
}

impl<T: Pod> PodExt for T {}

pub trait IdExt {
    fn id() -> TypeId;
    /// returns name of a type without full path
    fn struct_name() -> &'static str;
}

impl<T: 'static> IdExt for T {
    fn id() -> TypeId {
        TypeId::of::<Self>()
    }

    fn struct_name() -> &'static str {
        let name = std::any::type_name::<T>();
        let start = match name.rfind(':') {
            None => 0,
            Some(i) => i + 1,
        };
        &name[start..]
    }
}

#[async_trait]
pub trait Extend<A>: std::iter::Extend<A> {
    /// Extends collection if error occurrs in the middle then items aren't removed.
    async fn try_extend_stream<'a, E, T: Stream<Item = Result<A, E>> + Send + 'a>(
        &'a mut self,
        stream: T,
    ) -> Result<(), E> {
        pin!(stream);
        let (min, max) = stream.size_hint();
        if let Some(max) = max {
            self.extend_reserve(max);
        } else {
            self.extend_reserve(min);
        }
        while let Some(item) = stream.next().await {
            self.extend_one(item?);
        }
        Ok(())
    }

    async fn extend_stream<'a, C: Extend<A>, T: Stream<Item = A> + Send + 'a>(
        &'a mut self,
        mut stream: Pin<&mut T>,
    ) {
        let (min, max) = stream.size_hint();
        if let Some(max) = max {
            self.extend_reserve(max);
        } else {
            self.extend_reserve(min);
        }
        while let Some(item) = stream.next().await {
            self.extend_one(item);
        }
    }
}

// pub trait DynClone: dyn_clone::DynClone {
//    fn clone_box(&self) -> Box<Self> {
//        dyn_clone::clone_box(&self)
//    }
//}
// impl<T: ?Sized + dyn_clone::DynClone> DynClone for T {}

#[async_trait]
impl<A, E: std::iter::Extend<A>> Extend<A> for E {}
