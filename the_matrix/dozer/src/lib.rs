#![feature(try_blocks)]
#![feature(associated_type_bounds)]
#![feature(async_stream)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(option_result_unwrap_unchecked)]
#![deny(unused_must_use)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate futures_util;
#[macro_use]
extern crate pin_project;

use std::io::SeekFrom;

use async_compression::tokio::bufread::ZstdDecoder;
use mouse::traits::AsyncReadSeek;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

pub mod loaders;
pub mod provider;

#[async_trait]
pub trait SeekableDecoder {
    async fn read_exact(&mut self, data: &mut [u8]) -> std::io::Result<usize>;
    async fn read_to_end(&mut self, data: &mut Vec<u8>) -> std::io::Result<usize>;
    async fn read_exact_uncompressed(&mut self, data: &mut [u8]) -> std::io::Result<usize>;
    async fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64>;
}

#[async_trait]
impl SeekableDecoder for Box<dyn SeekableDecoder + Sync + Send + 'static> {
    async fn read_exact(&mut self, data: &mut [u8]) -> std::io::Result<usize> {
        self.as_mut().read_exact(data).await
    }
    async fn read_to_end(&mut self, data: &mut Vec<u8>) -> std::io::Result<usize> {
        self.as_mut().read_to_end(data).await
    }
    async fn read_exact_uncompressed(&mut self, data: &mut [u8]) -> std::io::Result<usize> {
        self.as_mut().read_exact_uncompressed(data).await
    }
    async fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.as_mut().seek(pos).await
    }
}

pub trait NestedStream {
    type Inner;
    fn get_mut(&mut self) -> &mut Self::Inner;
    fn into_inner(self) -> Self::Inner;
    fn from_inner(inner: Self::Inner) -> Self;
}

macro_rules! impl_nested_stream {
    ($ty:ident $(, $req:tt)?) => {
        impl<T $(: $req)?> NestedStream for $ty<T> {
            type Inner = T;

            fn get_mut(&mut self) -> &mut Self::Inner {
                self.get_mut()
            }

            fn into_inner(self) -> Self::Inner {
                self.into_inner()
            }

            fn from_inner(inner: Self::Inner) -> Self {
                Self::new(inner)
            }
        }
    };
}

impl_nested_stream!(ZstdDecoder, AsyncBufRead);
use tokio::io::BufReader;
impl_nested_stream!(BufReader, AsyncRead);

pub struct Decoder<R> {
    reader: Option<R>,
}

impl<R> Decoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: Some(reader),
        }
    }
}

#[async_trait]
impl<R: NestedStream<Inner: AsyncReadSeek + Unpin + Send> + AsyncReadSeek + Unpin + Send>
    SeekableDecoder for Decoder<R>
{
    async fn read_exact(&mut self, data: &mut [u8]) -> std::io::Result<usize> {
        self.reader.as_mut().unwrap().read_exact(data).await
    }

    async fn read_to_end(&mut self, data: &mut Vec<u8>) -> std::io::Result<usize> {
        self.reader.as_mut().unwrap().read_to_end(data).await
    }

    async fn read_exact_uncompressed(&mut self, data: &mut [u8]) -> std::io::Result<usize> {
        self.reader
            .as_mut()
            .unwrap()
            .get_mut()
            .read_exact(data)
            .await
    }

    async fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let mut reader = self.reader.take().unwrap().into_inner();
        let result = reader.seek(pos).await;
        self.reader = Some(R::from_inner(reader));
        result
    }
}
