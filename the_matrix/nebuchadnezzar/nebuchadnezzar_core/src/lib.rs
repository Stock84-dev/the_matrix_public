#![deny(unused_must_use)]
#![feature(async_closure)]
#![feature(async_stream)]
#![feature(try_blocks)]
#![feature(fn_traits)]
#![feature(unboxed_closures)]

pub mod client;
pub mod commands;
pub mod definitions;
pub mod error;
pub mod paginators;
pub mod requests;
#[cfg(feature = "schema")]
pub mod schema;
pub mod serializers;
pub mod signatures;
pub mod websocket;

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate pin_project;
#[macro_use]
pub extern crate thiserror;
#[macro_use]
extern crate log as macro_log;

pub mod prelude {
    pub use chrono::{DateTime, Utc};
    pub use reqwest::Method;
    pub use rust_decimal::prelude::Decimal;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value;
    pub use uuid::Uuid;

    pub use crate::client::Request;
}

pub mod reqwest {
    pub use reqwest::{Client as ReqwestClient, *};
}

use std::pin::Pin;

pub use async_trait::async_trait;

use futures_util::StreamExt;
pub use {chrono, futures_util, log, serde, serde_json, sorted_vec, tokio};

// pub use tokio_tungstenite;
use crate::client::{
    Client, ClientCapability, Converter, NotClient, NotRequest, NotResponse, Pageable, SuperClient,
    SuperRequest,
};
use crate::commands::*;

use crate::error::{AnyResult, NebError, Result};
use crate::paginators::{ConvertingPaginatorStream, Paginator, SuperPaginatorStream};
use crate::requests::*;
use crate::websocket::{
    CommandConverter, MessageConverter, NotWebSocket, SuperMessage, SuperWebSocket, WebSocket,
    WebSocketCapability,
};

#[derive(Clone, Debug)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
}

impl Credentials {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Credentials {
        Credentials {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }
}

#[async_trait]
pub trait SuperExchange {
    fn name(&self) -> &'static str;
    fn api_version(&self) -> &'static str;
    fn site_url(&self) -> &'static str;
    fn api_url(&self) -> &'static str;
    fn ws_api_url(&self) -> &'static str;
    fn api_doc_url(&self) -> &'static str;
    fn is_demo(&self) -> bool;
    fn new_client_dyn(&self) -> Box<dyn SuperClient>;
    fn client_capability(&self) -> ClientCapability;
    fn web_socket_capability(&self) -> WebSocketCapability;
    async fn new_web_socket_dyn(&self) -> AnyResult<Box<dyn SuperWebSocket>>;
}

#[derive(Clone, Copy, Debug)]
pub enum Support {
    No,
    Yes,
    Emulated,
}

impl Default for Support {
    fn default() -> Self {
        Support::No
    }
}

pub mod timeframes {
    #![allow(non_upper_case_globals)]
    pub const s1: u32 = 1;
    pub const m1: u32 = 60;
    pub const m5: u32 = 5 * 60;
    pub const h1: u32 = 60 * 60;
    pub const d1: u32 = 24 * 60 * 60;
    pub const w1: u32 = 7 * 24 * 60 * 60;
    pub const M1: u32 = 30 * 24 * 60 * 60;
    pub const y1: u32 = 365 * 24 * 60 * 60;
}

macro_rules! def_fetch {
    ($name:ident, $req:ty) => {
        fn $name<'life0, 'async_trait>(
            &'life0 self,
            req: $req,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<
                        Output = AnyResult<<$req as SuperRequest>::SuperResponse>,
                    > + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                let response = self.request(Self::convert_request(req)?).await?;
                Ok(<Self as Converter<$req>>::convert_response(response))
            })
        }
    };
}

macro_rules! def_paginate {
    ($name:ident, $req:ty) => {
        fn $name<'c: 'p, 'p>(
            &'c self,
            paginator: Pin<Box<dyn Paginator<$req, <$req as SuperRequest>::SuperResponse> + 'p>>,
        ) -> SuperPaginatorStream<'p, $req> {
            SuperPaginatorStream::new(Box::pin(ConvertingPaginatorStream::new(self, paginator)))
        }
    };
}

macro_rules! def_and_impl_traits_bounded_by {
    (($($super_request:ident)+), ($($command:ident)+)) => {
        #[async_trait]
        pub trait Exchange: 'static
        where
            Self::Client: Client<Exchange = Self> + Sync $(+ Converter<$super_request>)+,
            Self::WebSocket: WebSocket<Exchange = Self> + Send,
        {
            type Client;
            type WebSocket;

            fn name(&self) -> &'static str;
            fn api_version(&self) -> &'static str;
            fn site_url(&self) -> &'static str;
            fn api_url(&self) -> &'static str;
            fn ws_api_url(&self) -> &'static str;
            fn api_doc_url(&self) -> &'static str;
            fn is_demo(&self) -> bool;

            fn new_client(&self) -> Self::Client;
            async fn new_web_socket(&self) -> Result<Self::WebSocket, <Self::WebSocket as WebSocket>::Error>;

            fn client_capability() -> ClientCapability {
                Self::Client::capability()
            }

            fn web_socket_capability() -> WebSocketCapability {
                Self::WebSocket::capability()
            }
        }

        #[async_trait]
        impl<E: Exchange> SuperExchange for E
        where
            E: Exchange + Sync,
            $(<E::Client as Converter<$super_request>>::Req: Pageable,)+
            E::WebSocket: MessageConverter<Msg = <E::WebSocket as WebSocket>::Message>
                $(+ CommandConverter<$command>)+,
        {
            fn name(&self) -> &'static str {
                Exchange::name(self)
            }

            fn api_version(&self) -> &'static str {
                Exchange::api_version(self)
            }

            fn site_url(&self) -> &'static str {
                Exchange::site_url(self)
            }

            fn api_url(&self) -> &'static str {
                Exchange::api_url(self)
            }

            fn ws_api_url(&self) -> &'static str {
                Exchange::ws_api_url(self)
            }

            fn api_doc_url(&self) -> &'static str {
                Exchange::api_doc_url(self)
            }

            fn is_demo(&self) -> bool {
                Exchange::is_demo(self)
            }

            fn client_capability(&self) -> ClientCapability {
                <E::Client as Client>::capability()
            }

            fn web_socket_capability(&self) -> WebSocketCapability {
                <E::WebSocket as WebSocket>::capability()
            }

            fn new_client_dyn(&self) -> Box<dyn SuperClient> {
                Box::new(self.new_client())
            }

            async fn new_web_socket_dyn(&self) -> AnyResult<Box<dyn SuperWebSocket>> {
                match self.new_web_socket().await {
                    Ok(w) => Ok(Box::new(w)),
                    Err(e) => Err(e.into()),
                }
            }
        }

        #[async_trait]
        impl<C> SuperClient for C
        where
            C::Exchange: Sync,
            C: Client + Sync + Send $(+ Converter<$super_request>)+,
            $(
                <C as Converter<$super_request>>::Req: Pageable,
                <<C::Exchange as Exchange>::Client as Converter<$super_request>>::Req: Pageable,
            )+
            <C::Exchange as Exchange>::WebSocket: MessageConverter<Msg = <<C::Exchange as Exchange>::WebSocket as WebSocket>::Message>
                $(+ CommandConverter<$command>)+,
        {
            fn capability(&self) -> ClientCapability {
                Self::capability()
            }

            fn exchange_dyn(&self) -> Box<dyn SuperExchange> {
                Box::new(self.exchange())
            }

            fn authenticate(&mut self, credentials: Credentials) -> AnyResult<()> {
                Ok(<Self as Client>::authenticate(self, credentials)?)
            }

            def_fetch!(fetch_candles, CandlesGetRequest);
            def_fetch!(fetch_trades, TradesGetRequest);
            def_paginate!(paginate_candles, CandlesGetRequest);
            def_paginate!(paginate_trades, TradesGetRequest);
        }

        #[async_trait]
        impl<W> SuperWebSocket for W
        where
            W::Exchange: Sync,
            W: WebSocket
                + MessageConverter<Msg = <Self as WebSocket>::Message>
                + Send
                $(+ CommandConverter<$command>)+,
            $(<<W::Exchange as Exchange>::Client as Converter<$super_request>>::Req: Pageable,)+
        {
            fn exchange_dyn(&self) -> Box<dyn SuperExchange> {
                Box::new(self.exchange())
            }

            fn capability(&self) -> WebSocketCapability {
                Self::capability()
            }

            async fn authenticate(&mut self, credentials: Credentials) -> AnyResult<()> {
                <Self as WebSocket>::authenticate(self, credentials).await?;
                Ok(())
            }

            async fn next(&mut self) -> Option<AnyResult<SuperMessage>> {
                WebSocket::next(self).await.map(|x| match x {
                    Ok(message) => Ok(Self::convert_message(message)),
                    Err(e) => Err(e.into()),
                })
            }

            async fn ping(&mut self) -> AnyResult<()> {
                self.send(Self::convert_command(PingSuperCommand)).await?;
                Ok(())
            }
        }
        $(
            impl<E> Converter<$super_request> for NotClient<E>
            where
                E: Exchange<Client = NotClient<E>> + Sync + Send,
            {
                type Req = NotRequest;

                fn convert_request(_: $super_request) -> Result<Self::Req, NebError> {
                    unreachable!("This is not a Converter.")
                }

                fn convert_response(_: NotResponse) -> <$super_request as SuperRequest>::SuperResponse {
                    unreachable!("This is not a Converter.")
                }
            }
        )+
    };
}

def_and_impl_traits_bounded_by!((CandlesGetRequest TradesGetRequest), (PingSuperCommand));

pub struct NotExchange(());
#[async_trait]
impl Exchange for NotExchange {
    type Client = NotClient<Self>;
    type WebSocket = NotWebSocket<Self>;

    fn name(&self) -> &'static str {
        unreachable!()
    }

    fn api_version(&self) -> &'static str {
        unreachable!()
    }

    fn site_url(&self) -> &'static str {
        unreachable!()
    }

    fn api_url(&self) -> &'static str {
        unreachable!()
    }

    fn ws_api_url(&self) -> &'static str {
        unreachable!()
    }

    fn api_doc_url(&self) -> &'static str {
        unreachable!()
    }

    fn is_demo(&self) -> bool {
        unreachable!()
    }

    fn new_client(&self) -> Self::Client {
        unreachable!()
    }

    async fn new_web_socket(
        &self,
    ) -> Result<Self::WebSocket, <Self::WebSocket as WebSocket>::Error> {
        unreachable!()
    }
}
