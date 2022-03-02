use mouse::error::Result;
mod hlcv_provider;
pub use hlcv_provider::*;

#[async_trait]
pub trait OwnedProvider {
    type Item: Send + Clone;
    async fn provide(&mut self) -> Result<Option<Self::Item>>;
    fn index(&self) -> usize;
}

#[async_trait]
pub trait Provider {
    type Item: Send + Clone;
    async fn provide_ref<'a>(&'a mut self) -> Result<Option<&'a Self::Item>>;
    fn index(&self) -> usize;
}

#[async_trait]
impl<T: Provider + Send> OwnedProvider for T {
    type Item = T::Item;

    async fn provide(&mut self) -> Result<Option<Self::Item>> {
        self.provide_ref().await.map(|x| x.map(|x| x.clone()))
    }

    fn index(&self) -> usize {
        self.index()
    }
}

pub struct CachingProvider<P: OwnedProvider> {
    cache: Vec<P::Item>,
    provider: P,
}

impl<P: OwnedProvider> CachingProvider<P> {
    pub fn new(provider: P, capacity: usize) -> CachingProvider<P> {
        CachingProvider {
            cache: Vec::with_capacity(capacity),
            provider,
        }
    }
}

#[async_trait]
impl<P: OwnedProvider + Send> Provider for CachingProvider<P> {
    type Item = P::Item;

    async fn provide_ref<'a>(&'a mut self) -> Result<Option<&'a Self::Item>> {
        if let Some(item) = self.provider.provide().await? {
            let index = self.cache.len();
            self.cache.push(item);
            unsafe {
                return Ok(Some(self.cache.get_unchecked(index)));
            }
        }
        Ok(None)
    }

    fn index(&self) -> usize {
        self.provider.index()
    }
}

pub struct CacheProvider<P: OwnedProvider> {
    cache: Vec<P::Item>,
    provider: P,
    index: usize,
}

impl<P: Provider + Send> CacheProvider<P> {
    pub fn new(caching_provider: CachingProvider<P>) -> CacheProvider<P> {
        CacheProvider {
            cache: caching_provider.cache,
            provider: caching_provider.provider,
            index: 0,
        }
    }

    pub fn into_parts(self) -> (P, Vec<P::Item>) {
        (self.provider, self.cache)
    }
}

#[async_trait]
impl<P: OwnedProvider + Send> Provider for CacheProvider<P> {
    type Item = P::Item;

    async fn provide_ref<'a>(&'a mut self) -> Result<Option<&'a Self::Item>> {
        if self.index >= self.cache.len() {
            return Ok(None);
        }
        let item = unsafe { self.cache.get_unchecked(self.index) };
        self.index += 1;
        Ok(Some(item))
    }

    fn index(&self) -> usize {
        self.index
    }
}

impl<P: Provider + Send> AsRef<P> for CacheProvider<P> {
    fn as_ref(&self) -> &P {
        &self.provider
    }
}

impl<P: Provider + Send> AsMut<P> for CacheProvider<P> {
    fn as_mut(&mut self) -> &mut P {
        &mut self.provider
    }
}
