use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite};

pub trait Builder<Args> {
    type Target;
    type Err;
    fn build(self, args: Args) -> Result<Self::Target, Self::Err>;
}

pub trait RefBuilder<Args> {
    type Target;
    type Err;
    fn build(&self, args: Args) -> Result<Self::Target, Self::Err>;
}

pub trait MutBuilder<Args> {
    type Target;
    type Err;
    fn build(&mut self, args: Args) -> Result<Self::Target, Self::Err>;
}

impl<Args, T: RefBuilder<Args>> MutBuilder<Args> for T {
    type Target = T::Target;
    type Err = T::Err;

    fn build(&mut self, args: Args) -> Result<Self::Target, Self::Err> {
        RefBuilder::build(self, args)
    }
}

pub trait AsyncReadSeek: AsyncRead + AsyncSeek + Send + Sync + 'static {}
pub trait AsyncWriteSeek: AsyncWrite + AsyncSeek + Send + Sync {}
pub trait AsyncRwSeek: AsyncRead + AsyncWrite + AsyncSeek + Send + Sync {}

impl<T: AsyncRead + AsyncSeek + Send + Sync + 'static> AsyncReadSeek for T {}
impl AsyncReadSeek for Pin<Box<dyn AsyncReadSeek>> {}

impl<T: AsyncWrite + AsyncSeek + Send + Sync + 'static> AsyncWriteSeek for T {}
