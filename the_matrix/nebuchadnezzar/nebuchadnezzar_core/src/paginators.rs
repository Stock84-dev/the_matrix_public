use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::ready;

use crate::client::{Client, Converter, Pageable, Request, SuperRequest};
use crate::error::{AnyResult};
use crate::futures_util::Stream;

#[derive(Clone, Debug)]
pub struct BasicPaginatorState {
    pub i: u32,
    pub count: u32,
    pub end: u32,
    pub stride: u32,
    pub max_page_size: u32,
}

impl BasicPaginatorState {
    fn new(start: u32, end: u32, stride: u32, max_page_size: u32) -> BasicPaginatorState {
        BasicPaginatorState {
            i: start,
            count: 0,
            end,
            stride,
            max_page_size,
        }
    }

    fn start_generate(&mut self) -> Option<()> {
        if self.i >= self.end {
            return None;
        }
        self.count = ((self.end - self.i) / self.stride).min(self.max_page_size);
        if self.count == 0 {
            return None;
        }
        Some(())
    }

    fn finish_generate<R>(&mut self, response: &AnyResult<R>) {
        if response.is_ok() {
            self.i += self.count * self.stride;
        }
    }

    #[inline]
    fn size_hint(&self) -> usize {
        let elements = (self.end - self.i) / self.stride;
        (elements / self.max_page_size + (elements % self.max_page_size == 0) as u32) as usize
    }
}

pub trait Paginator<R, P>: Stream<Item = AnyResult<R>> + Send {
    fn set_max_items_per_page(self: Pin<&mut Self>, size: u32);
    /// Gets called when client receives response
    fn on_page(self: Pin<&mut Self>, page: &AnyResult<P>);
}

#[pin_project]
pub struct WhilePaginator<C, G, R> {
    generator: G,
    request: Option<AnyResult<R>>,
    max_items_per_page: u32,
    _c: PhantomData<C>,
}

impl<C, R, G> WhilePaginator<C, G, R>
where
    C: Client,
    R: Request<C> + Pageable,
    for<'s> G: FnMut(&'s AnyResult<R::Response>, u32) -> AnyResult<R> + Send,
{
    pub fn new(first_request: AnyResult<R>, generator: G) -> Self {
        Self {
            generator,
            request: Some(first_request),
            max_items_per_page: 0,
            _c: Default::default(),
        }
    }
}

impl<C, R, G> Stream for WhilePaginator<C, G, R>
where
    C: Client,
    R: Request<C> + Pageable,
    for<'s> G: FnMut(&'s AnyResult<R::Response>, u32) -> Option<AnyResult<R>> + Send,
{
    type Item = AnyResult<R>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.request.take() {
            None => Poll::Ready(None),
            Some(req) => Poll::Ready(Some(req)),
        }
    }
}

impl<C, R, G> Paginator<R, R::Response> for WhilePaginator<C, G, R>
where
    C: Client,
    R: Request<C> + Pageable + Sync,
    R::Response: Sync,
    for<'s> G: FnMut(&'s AnyResult<R::Response>, u32) -> Option<AnyResult<R>> + Send,
{
    fn set_max_items_per_page(mut self: Pin<&mut Self>, size: u32) {
        self.max_items_per_page = size;
    }

    fn on_page(mut self: Pin<&mut Self>, response: &AnyResult<R::Response>) {
        let max_items_per_page = self.max_items_per_page;
        self.request = (self.generator)(response, max_items_per_page);
    }
}

#[pin_project]
pub struct WhileSuperPaginator<G, SR> {
    generator: G,
    request: Option<AnyResult<SR>>,
    max_items_per_page: u32,
}

impl<SR, G> WhileSuperPaginator<G, SR>
where
    SR: SuperRequest,
    for<'s> G: FnMut(&'s AnyResult<SR::SuperResponse>, u32) -> Option<AnyResult<SR>> + Send,
{
    pub fn new(first_request: AnyResult<SR>, generator: G) -> Self {
        Self {
            generator,
            request: Some(first_request),
            max_items_per_page: 0,
        }
    }
}

impl<SR, G> Stream for WhileSuperPaginator<G, SR>
where
    SR: SuperRequest,
    for<'s> G: FnMut(&'s AnyResult<SR::SuperResponse>, u32) -> Option<AnyResult<SR>> + Send,
{
    type Item = AnyResult<SR>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.request.take() {
            None => Poll::Ready(None),
            Some(req) => Poll::Ready(Some(req)),
        }
    }
}

impl<SR, G> Paginator<SR, SR::SuperResponse> for WhileSuperPaginator<G, SR>
where
    SR: SuperRequest,
    for<'s> G: FnMut(&'s AnyResult<SR::SuperResponse>, u32) -> Option<AnyResult<SR>> + Send,
{
    fn set_max_items_per_page(mut self: Pin<&mut Self>, size: u32) {
        self.max_items_per_page = size;
    }

    fn on_page(mut self: Pin<&mut Self>, response: &AnyResult<SR::SuperResponse>) {
        let max_items_per_page = self.max_items_per_page;
        self.request = (self.generator)(response, max_items_per_page);
    }
}

#[pin_project]
pub struct BasicPaginator<C, G> {
    generator: G,
    state: BasicPaginatorState,
    _c: PhantomData<C>,
}

impl<C, R, G> BasicPaginator<C, G>
where
    for<'s> G: FnMut(&'s BasicPaginatorState) -> AnyResult<R> + Send,
{
    pub fn new(start: u32, end: u32, stride: u32, generator: G) -> Self {
        Self {
            generator,
            state: BasicPaginatorState::new(start, end, stride, 0),
            _c: Default::default(),
        }
    }

    fn next(&mut self) -> Poll<Option<AnyResult<R>>> {
        match self.state.start_generate() {
            Some(()) => Poll::Ready(Some((self.generator)(&self.state))),
            None => Poll::Ready(None),
        }
    }
}

impl<C, R, G> Paginator<R, R::Response> for BasicPaginator<C, G>
where
    C: Client,
    R: Request<C> + Sync,
    R::Response: Sync,
    for<'s> G: FnMut(&'s BasicPaginatorState) -> AnyResult<R> + Send,
{
    fn set_max_items_per_page(mut self: Pin<&mut Self>, size: u32) {
        self.state.max_page_size = size;
    }

    fn on_page(self: Pin<&mut Self>, response: &AnyResult<R::Response>) {
        self.project().state.finish_generate(response);
    }
}

impl<C, R, G> Stream for BasicPaginator<C, G>
where
    C: Client,
    R: Request<C>,
    for<'s> G: FnMut(&'s BasicPaginatorState) -> AnyResult<R> + Send,
{
    type Item = AnyResult<R>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.state.size_hint(), None)
    }
}

#[pin_project]
pub struct BasicSuperPaginator<G> {
    generator: G,
    state: BasicPaginatorState,
}

impl<SR, G> BasicSuperPaginator<G>
where
    SR: SuperRequest + Sync,
    SR::SuperResponse: Sync,
    for<'s> G: FnMut(&'s BasicPaginatorState) -> AnyResult<SR> + Send,
{
    pub fn new(start: u32, end: u32, stride: u32, generator: G) -> Self {
        Self {
            generator,
            state: BasicPaginatorState::new(start, end, stride, 0),
        }
    }

    fn next(&mut self) -> Poll<Option<AnyResult<SR>>> {
        match self.state.start_generate() {
            Some(()) => Poll::Ready(Some((self.generator)(&self.state))),
            None => Poll::Ready(None),
        }
    }
}

// impl<C, R, G> BasicPaginator<C, G>
// where
//     C: Client,
//     R: Request<C> + Sync,
//     R::Response: Sync,
//     G: FnMut(&BasicPaginatorState) -> AnyResult<R> + Send,
// {
//     pub fn as_paginator(&mut self) -> &mut dyn Paginator<R, R::Response> {
//         self
//     }
// }
//
// impl<SR, G> BasicPaginator<(), G>
// where
//     SR: SuperRequest + Sync,
//     SR::SuperResponse: Sync,
//     G: FnMut(&BasicPaginatorState) -> AnyResult<SR> + Send,
//     BasicPaginator<(), G>: Stream<Item = AnyResult<SR>>,
// {
//     pub fn as_paginator(&mut self) -> &mut dyn Paginator<SR, SR::SuperResponse> {
//         self
//     }
// }

impl<SR, G> Paginator<SR, SR::SuperResponse> for BasicSuperPaginator<G>
where
    SR: SuperRequest + Sync,
    SR::SuperResponse: Sync,
    for<'s> G: FnMut(&'s BasicPaginatorState) -> AnyResult<SR> + Send,
    BasicSuperPaginator<G>: Stream<Item = AnyResult<SR>>,
{
    fn set_max_items_per_page(mut self: Pin<&mut Self>, size: u32) {
        self.state.max_page_size = size;
    }

    fn on_page(self: Pin<&mut Self>, response: &AnyResult<SR::SuperResponse>) {
        self.project().state.finish_generate(response);
    }
}

impl<SR, G> Stream for BasicSuperPaginator<G>
where
    SR: SuperRequest,
    SR::SuperResponse: Sync,
    for<'s> G: FnMut(&'s BasicPaginatorState) -> AnyResult<SR> + Send,
{
    type Item = AnyResult<SR>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.state.size_hint(), None)
    }
}

pub struct PaginatorStream<'c, 'p, C: Client, R: Request<C> + 'c, P: Paginator<R, R::Response>> {
    // pin_project cannot be used with structs that hold mutable references due to lifetime
    // inference of poll_next
    // https://github.com/taiki-e/pin-project/issues/226
    client: &'c C,
    paginator: Pin<&'p mut P>,
    // Doesn't need to be pinned on stack becaue the future is generated from trait
    request_fut: Option<Pin<Box<dyn Future<Output = AnyResult<R::Response>> + Send + 'c>>>,
}

impl<'c, 'p, C, R, P> PaginatorStream<'c, 'p, C, R, P>
where
    C: Client + Sync + 'c,
    R: Request<C> + Pageable,
    P: Paginator<R, R::Response>,
{
    pub fn new(client: &'c C, mut paginator: Pin<&'p mut P>) -> Self {
        paginator
            .as_mut()
            .set_max_items_per_page(R::MAX_ITEMS_PER_PAGE);
        Self {
            client,
            paginator,
            request_fut: None,
        }
    }
}

impl<'c, 'p, C, R, P> Stream for PaginatorStream<'c, 'p, C, R, P>
where
    C: Client + Sync + 'c,
    R: Request<C>,
    P: Paginator<R, R::Response>,
{
    type Item = AnyResult<R::Response>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &mut self.request_fut {
                None => match ready!(self.paginator.as_mut().poll_next(cx)) {
                    None => return Poll::Ready(None),
                    Some(Ok(request)) => {
                        self.request_fut = Some(self.client.request(request));
                    }
                    Some(Err(e)) => return Poll::Ready(Some(Err(e.into()))),
                },
                Some(fut) => {
                    let result = ready!(fut.as_mut().poll(cx));
                    self.request_fut = None;
                    self.paginator.as_mut().on_page(&result);
                    return Poll::Ready(Some(result.map_err(|e| e.into())));
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.paginator.size_hint()
    }
}

pub struct ConvertingPaginatorStream<
    'c,
    C: Converter<SR> + Sync,
    SR: SuperRequest + 'c,
    // P: Paginator<SR, SR::SuperResponse>,
> {
    // pin_project cannot be used with structs that hold mutable references due to lifetime
    // inference of poll_next
    // https://github.com/taiki-e/pin-project/issues/226
    client: &'c C,
    // #[pin]
    paginator: Pin<Box<dyn Paginator<SR, SR::SuperResponse> + 'c>>,
    // Doesn't need to be pinned on stack becaue the future is generated from trait
    request_fut: Option<
        Pin<Box<dyn Future<Output = AnyResult<<C::Req as Request<C>>::Response>> + Send + 'c>>,
    >,
}

impl<'c, C, SR> ConvertingPaginatorStream<'c, C, SR>
where
    C: Converter<SR> + Sync,
    SR: SuperRequest + 'c,
    C::Req: Pageable,
    // P: Paginator<SR, SR::SuperResponse>,
{
    pub fn new(
        client: &'c C,
        mut paginator: Pin<Box<dyn Paginator<SR, SR::SuperResponse> + 'c>>,
    ) -> Self {
        paginator
            .as_mut()
            .set_max_items_per_page(C::Req::MAX_ITEMS_PER_PAGE);
        Self {
            client,
            paginator: paginator,
            request_fut: None,
        }
    }
}

impl<'c, C, SR> Stream for ConvertingPaginatorStream<'c, C, SR>
where
    C: Converter<SR> + Sync,
    SR: SuperRequest + 'c,
    C::Req: Pageable,
{
    type Item = AnyResult<SR::SuperResponse>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            trace!("poll stream");
            match &mut self.request_fut {
                None => match ready!(self.paginator.as_mut().poll_next(cx)) {
                    None => return Poll::Ready(None),
                    Some(Ok(request)) => match C::convert_request(request) {
                        Ok(request) => {
                            self.request_fut = Some(self.client.request(request));
                        }
                        Err(e) => return Poll::Ready(Some(Err(e.into()))),
                    },
                    Some(Err(e)) => return Poll::Ready(Some(Err(e.into()))),
                },
                Some(fut) => {
                    let result = ready!(fut.as_mut().poll(cx));
                    trace!("stream polled");
                    self.request_fut = None;
                    let result = result.map(|response| C::convert_response(response));
                    self.paginator.as_mut().on_page(&result);
                    return Poll::Ready(Some(result.map_err(|e| e.into())));
                }
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.paginator.size_hint()
    }
}

pub struct SuperPaginatorStream<'s, SR: SuperRequest> {
    inner: Pin<Box<dyn Stream<Item = AnyResult<SR::SuperResponse>> + Send + 's>>,
}

impl<'s, SR: SuperRequest> SuperPaginatorStream<'s, SR> {
    pub fn new(
        inner: Pin<Box<dyn Stream<Item = AnyResult<SR::SuperResponse>> + Send + 's>>,
    ) -> Self {
        Self { inner }
    }
}

impl<'s, SR: SuperRequest> Stream for SuperPaginatorStream<'s, SR> {
    type Item = AnyResult<SR::SuperResponse>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
