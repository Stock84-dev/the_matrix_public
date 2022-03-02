use std::fmt::Debug;
use std::marker::PhantomData;
use std::pin::Pin;

use async_trait::async_trait;

use reqwest::{Response, Url};
pub use ring;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sorted_vec::SortedSet;


use crate::error::{AnyResult, NebError, RemoteError, Result};
use crate::paginators::{Paginator, PaginatorStream, SuperPaginatorStream};
use crate::requests::{CandlesGetRequest, TradesGetRequest};
use crate::reqwest::Method;
use crate::{Credentials, Exchange, SuperExchange, Support};

#[derive(Default, Clone, Debug)]
pub struct ClientCapability {
    pub timeframes: SortedSet<u32>,
    pub cors: Support,
    pub cancel_order: Support,
    pub create_deposit_address: Support,
    pub create_order: Support,
    pub deposit: Support,
    pub fetch_balance: Support,
    pub fetch_closed_orders: Support,
    pub fetch_currencies: Support,
    pub fetch_deposit_address: Support,
    pub fetch_markets: Support,
    pub fetch_my_trades: Support,
    pub fetch_candles: Support,
    pub fetch_open_orders: Support,
    pub fetch_order: Support,
    pub fetch_order_book: Support,
    pub fetch_orders: Support,
    pub fetch_status: Support,
    pub fetch_ticker: Support,
    pub fetch_tickers: Support,
    pub fetch_bids_asks: Support,
    pub fetch_trades: Support,
    pub withdraw: Support,
}

pub trait Pageable: Sized {
    const MAX_ITEMS_PER_PAGE: u32;
}

pub trait SuperRequest: Clone + Debug + Send + Sync + 'static {
    type SuperResponse: Send;
}

pub trait Request<C: Client>: Serialize + Send {
    const METHOD: Method;
    const SIGNED: bool;
    const ENDPOINT: &'static str;
    type Response: DeserializeOwned;
}

trait ToUrlQuery: Serialize {
    fn to_url_query_string(&self) -> String {
        let vec = self.to_url_query();
        vec.into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }

    fn to_url_query(&self) -> Vec<(String, String)> {
        let v = serde_json::to_value(self).unwrap();
        let v = v.as_object().unwrap();
        let mut vec = vec![];

        for (key, value) in v.into_iter() {
            if value.is_null() {
                continue;
            } else if value.is_string() {
                vec.push((key.clone(), value.as_str().unwrap().to_string()))
            } else {
                vec.push((key.clone(), serde_json::to_string(value).unwrap()))
            }
        }
        vec
    }
}
impl<S: Serialize> ToUrlQuery for S {}

#[async_trait]
pub trait Client: Send {
    type Exchange: Exchange;
    fn exchange(&self) -> Self::Exchange;
    fn authenticate(&mut self, credentials: Credentials) -> AnyResult<()>;
    fn capability() -> ClientCapability;
    async fn request_raw<R>(&self, url: Url, body: String) -> AnyResult<R::Response>
    where
        Self: Sized,
        R: Request<Self>,
        R::Response: DeserializeOwned;

    async fn request<R>(&self, req: R) -> AnyResult<R::Response>
    where
        Self: Sized,
        R: Request<Self>,
        R::Response: DeserializeOwned,
    {
        let url = format!("{}{}", self.exchange().api_url(), R::ENDPOINT);
        let url = match R::METHOD {
            Method::GET | Method::DELETE => {
                if std::mem::size_of::<R>() != 0 {
                    Url::parse_with_params(&url, req.to_url_query())?
                } else {
                    Url::parse(&url)?
                }
            }
            _ => Url::parse(&url)?,
        };

        let body = match R::METHOD {
            Method::PUT | Method::POST if std::mem::size_of::<R>() != 0 => {
                serde_json::to_string(&req)?
            }
            _ => "".to_string(),
        };

        self.request_raw::<R>(url, body).await
    }

    fn paginate<'c, 'p, P, R>(
        &'c self,
        paginator: Pin<&'p mut P>,
    ) -> PaginatorStream<'c, 'p, Self, R, P>
    where
        Self: Sized + Sync,
        P: Paginator<R, R::Response>,
        R: Request<Self> + Pageable + 'c,
    {
        PaginatorStream::new(self, paginator)
    }
}

pub trait Converter<SR: SuperRequest>: Client + Sized {
    type Req: Request<Self>;
    fn convert_request(super_request: SR) -> Result<Self::Req, NebError>;
    fn convert_response(response: <Self::Req as Request<Self>>::Response) -> SR::SuperResponse;
}

#[async_trait]
pub trait SuperClient: Send {
    fn capability(&self) -> ClientCapability;
    fn exchange_dyn(&self) -> Box<dyn SuperExchange>;
    fn authenticate(&mut self, credentials: Credentials) -> AnyResult<()>;
    async fn fetch_candles(
        &self,
        req: CandlesGetRequest,
    ) -> AnyResult<<CandlesGetRequest as SuperRequest>::SuperResponse>;
    async fn fetch_trades(
        &self,
        req: TradesGetRequest,
    ) -> AnyResult<<TradesGetRequest as SuperRequest>::SuperResponse>;
    fn paginate_candles<'c: 'p, 'p>(
        &'c self,
        paginator: Pin<
            Box<
                dyn Paginator<
                        CandlesGetRequest,
                        <CandlesGetRequest as SuperRequest>::SuperResponse,
                        Item = AnyResult<CandlesGetRequest>,
                    > + 'p,
            >,
        >,
    ) -> SuperPaginatorStream<'p, CandlesGetRequest>;
    fn paginate_trades<'c: 'p, 'p>(
        &'c self,
        paginator: Pin<
            Box<
                dyn Paginator<
                        TradesGetRequest,
                        <TradesGetRequest as SuperRequest>::SuperResponse,
                        Item = AnyResult<TradesGetRequest>,
                    > + 'p,
            >,
        >,
    ) -> SuperPaginatorStream<'p, TradesGetRequest>;
}

// impl<C, SR, P> Paginator<C, C::Req, SR> for P
// where
//     C: Converter<SR>,
//     SR: SuperRequest,
//     P: SuperPaginator<SR>,
// {
//     type Output = SR::SuperResponse;
//
//     fn generate(&mut self) -> Option<Result<C::Req>> {
//         SuperPaginator::generate(self).map(|x| match x {
//             Ok(sr) => C::convert_request(sr),
//             Err(e) => Err(e),
//         })
//     }
//
//     fn validate(
//         &mut self,
//         response: Result<<C::Req as Request<C>>::Response>,
//     ) -> Option<Result<Self::Output>> {
//         match response {
//             Ok(r) => Some(Ok(C::convert_response(response))),
//             Err(e) => Some(Err(e)),
//         }
//     }
// }

pub async fn handle_response<T: DeserializeOwned>(resp: Response) -> Result<T> {
    return if resp.status().is_success() {
        let resp = resp.text().await?;
        // debug!("response body: |{}|", resp);
        match serde_json::from_str::<T>(&resp) {
            Ok(resp) => Ok(resp),
            Err(e) => {
                error!("Cannot deserialize '{}'", resp);
                error!("{:?}", e);
                Err(e.into())
            }
        }
    } else {
        Err(NebError::RemoteError(RemoteError::from(resp).await).into())
    };
}

#[derive(Deserialize)]
pub struct NotResponse(());

#[derive(Serialize)]
pub struct NotRequest(());

impl<C: Client> Request<C> for NotRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "";
    type Response = NotResponse;
}

pub struct NotClient<E>(PhantomData<E>);

#[async_trait]
impl<E: Exchange<Client = Self> + Sync + Send> Client for NotClient<E> {
    type Exchange = E;

    fn exchange(&self) -> Self::Exchange {
        unreachable!()
    }

    fn authenticate(&mut self, _: Credentials) -> AnyResult<()> {
        unreachable!()
    }

    fn capability() -> ClientCapability {
        unreachable!("This is not a Client.")
    }

    async fn request_raw<R>(&self, _: Url, _: String) -> AnyResult<<R as Request<Self>>::Response>
    where
        Self: Sized,
        R: Request<Self>,
        R::Response: DeserializeOwned,
    {
        unreachable!()
    }
}

#[macro_export]
macro_rules! forward_requests {
    ($kind:ty => $($client:ty),+) => {
        $(
            impl<R: Request<$kind>> Request<$client> for R {
                const METHOD: Method = R::METHOD;
                const SIGNED: bool = R::SIGNED;
                const ENDPOINT: &'static str = R::ENDPOINT;
                type Response = R::Response;
            }
        )+
    }
}

/// Implements Client for provided struct where first field implements Client.
/// ```ignore
/// derive_client! {
///     type Exchange = BitmexExchange;
///     pub struct BitmexClient {
///         inner: InnerClient,
///     }
/// }
/// ```
#[macro_export]
macro_rules! derive_client {
    (
        type Exchange = $exchange:ty;
        $(#[$attr:meta])*
        $struct_vis:vis struct $name:ident {
            $($(#[$field_attr:meta])* $field_vis:vis $field_name:ident : $field_type:ty),+
            $(,)?
        }
    ) => {
        $(#[$attr])*
        $struct_vis struct $name {
            $($(#[$field_attr])* $field_vis $field_name : $field_type),+
        }

        #[$crate::async_trait]
        impl $crate::client::Client for $name {
            type Exchange = $exchange;

            fn authenticate(
                &mut self,
                credentials: $crate::Credentials,
            ) -> $crate::error::Result<()> {
                derive_client!(self $($field_name)*,).authenticate(credentials)
            }

            fn capability() -> $crate::client::ClientCapability {
                <derive_client!(type $($field_type)*,)>::capability()
            }

            async fn request_raw<R>(
                &self,
                url: $crate::reqwest::Url,
                body: String,
            ) -> $crate::error::Result<R::Response>
            where
                Self: Sized,
                R: $crate::client::Request<Self>,
                R::Response: $crate::serde::de::DeserializeOwned,
            {
                derive_client!(self $($field_name)*,).request_raw::<Self, R>(url, body).await
            }
        }
    };
    ($self:ident $field_name:ident, $($other:tt)*) => {
        $self.$field_name
    };
    (type $field_type:ty, $($other:tt)*) => {
        $field_type
    };
    (name $(#[$field_attr:meta])* $field_vis:vis $field_name:ident : $field_type:ty, $($other:tt)*) => {
        $field_name
    };
    (type $(#[$field_attr:meta])* $field_vis:vis $field_name:ident : $field_type:ty, $($other:tt)*) => {
        $field_type
    };
}

/// Implements Request for every type in Request<$(types:ty),+>;
#[macro_export]
macro_rules! converter {
    (
        impl Request<$($clients:ty),+> for $request:ty {
            const METHOD: Method = $method:expr;
            const SIGNED: bool = $signed:expr;
            const ENDPOINT: &'static str = $endpoint:expr;
            type Response = $response:ty;
        }
    ) => {
         $(
            impl $crate::client::Request<$clients> for $request {
                const METHOD: $crate::reqwest::Method = $method;
                const SIGNED: bool = $signed;
                const ENDPOINT: &'static str = $endpoint;
                type Response = $response;
            }
        )+
    };
    (
        from $from:ty;
        impl Request<$($clients:ty),+> for $request:ty {
            const METHOD: Method = $method:expr;
            const SIGNED: bool = $signed:expr;
            const ENDPOINT: &'static str = $endpoint:expr;
            type Response = $response:ty;
        }
    ) => {
        $(
            impl $crate::client::Request<$clients> for $request {
                const METHOD: $crate::reqwest::Method = $method;
                const SIGNED: bool = $signed;
                const ENDPOINT: &'static str = $endpoint;
                type Response = $response;
            }
            impl Converter<$from> for $clients {
                type Req = $request;

                fn convert_request(super_request: $from) -> Result<Self::Req, NebError> {
                    use core::convert::TryInto;
                    Ok(super_request.try_into()?)
                }

                fn convert_response(
                    response: $response,
                ) -> <$from as SuperRequest>::SuperResponse {
                    response.into_iter().map(|x| x.into()).collect()
                }
            }
        )+
    }
}

#[macro_export]
macro_rules! impl_converter {
    (
        for $($clients:ty),+;
        $sr:ty => $request:block,
        $r:ty => $response:block $(,)?
    ) => {
        $(
            impl Converter<$sr> for $clients {
                type Req = $r;

                fn convert_request(super_request: requests::$sr) -> Result<Self::Req, NebError> {
                    $request
                }

                fn convert_response(response: $response) -> <$sr as SuperRequest>::SuperResponse {
                    $response
                }
            }
        )+
    }

}
