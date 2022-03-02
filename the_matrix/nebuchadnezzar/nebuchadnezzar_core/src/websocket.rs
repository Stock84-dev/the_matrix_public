use std::any::Any;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize};
pub use tokio::net::TcpStream;


pub use {futures_util, tokio_tungstenite};

use crate::error::{AnyResult, NebError, NotError, Result};

use crate::{async_trait, Credentials, Exchange, SuperExchange, Support};

#[derive(Default, Clone, Debug)]
pub struct WebSocketCapability {
    // Public
    pub load_markets: Support,
    pub watch_ticker: Support,
    pub watch_tickers: Support,
    pub watch_order_book: Support,
    pub watch_candles: Support,
    pub watch_status: Support,
    pub watch_trades: Support,

    // Private
    pub watch_balance: Support,
    pub watch_create_order: Support,
    pub watch_cancel_order: Support,
    pub watch_order: Support,
    pub watch_orders: Support,
    pub watch_open_orders: Support,
    pub watch_closed_orders: Support,
    pub watch_my_trades: Support,
    pub watch_deposit: Support,
    pub watch_withdraw: Support,
}

#[async_trait]
pub trait SuperWebSocket {
    fn exchange_dyn(&self) -> Box<dyn SuperExchange>;
    fn capability(&self) -> WebSocketCapability;
    async fn authenticate(&mut self, credentials: Credentials) -> AnyResult<()>;
    async fn next(&mut self) -> Option<AnyResult<SuperMessage>>;
    async fn ping(&mut self) -> AnyResult<()>;
}

pub trait SuperCommand {}

pub trait CommandConverter<C: SuperCommand>: WebSocket {
    type Command: WsCommand<Self>;
    fn convert_command(command: C) -> Self::Command;
}

pub trait MessageConverter: WebSocket {
    type Msg: WsMessage<Self>;
    fn convert_message(message: Self::Msg) -> SuperMessage;
}

#[derive(Debug)]
pub enum SuperMessage {
    Ping,
    Pong,
    Other(Box<dyn Any>),
}

pub trait WsMessage<WS: WebSocket + ?Sized>: DeserializeOwned + Send + Sync {}
pub trait WsCommand<WS: WebSocket + ?Sized>: Send + Sync {
    fn serialize(&self) -> WS::RawCommand;
}

#[async_trait]
pub trait WebSocket: Send + Sync {
    type Error: std::error::Error + Sync + Send + 'static;
    type Exchange: Exchange<WebSocket = Self>;
    type Message: WsMessage<Self>;
    type RawCommand;

    fn exchange(&self) -> Self::Exchange;
    fn capability() -> WebSocketCapability;
    async fn authenticate(&mut self, credentials: Credentials) -> Result<(), Self::Error>;

    async fn send<C>(&mut self, command: C) -> Result<(), Self::Error>
    where
        Self: Sized,
        C: WsCommand<Self>;

    async fn next(&mut self) -> Option<Result<Self::Message, Self::Error>>
    where
        Self: Sized;

    async fn close(&mut self) -> Result<(), Self::Error>;
}

pub struct NotWebSocket<E>(PhantomData<E>);

impl<E> NotWebSocket<E> {
    pub fn unsupported() -> Result<Self> {
        Err(NebError::Unsupported("websocket").into())
    }
}

#[derive(Deserialize)]
pub struct NotMessage(());

impl<W: WebSocket> WsMessage<W> for NotMessage {}

#[async_trait]
impl<E: Exchange<WebSocket = NotWebSocket<E>> + Sync + Send> WebSocket for NotWebSocket<E> {
    type Error = NotError;
    type Exchange = E;
    type Message = NotMessage;
    type RawCommand = ();

    fn exchange(&self) -> Self::Exchange {
        unreachable!()
    }

    fn capability() -> WebSocketCapability {
        unreachable!("This is not a WebSocket.")
    }

    async fn authenticate(&mut self, _: Credentials) -> Result<(), Self::Error> {
        unreachable!()
    }

    async fn send<C>(&mut self, _command: C) -> Result<(), Self::Error>
    where
        Self: Sized,
        C: WsCommand<Self>,
    {
        unreachable!()
    }

    async fn next(&mut self) -> Option<Result<Self::Message, Self::Error>>
    where
        Self: Sized,
    {
        unreachable!()
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        unreachable!()
    }
}

/// Implements WebSocket trait for provided struct.
/// ```ignore
/// derive_web_socket! {
///     type Exchange = BitmexExchange;
///     type Message = message::Message;
///     pub struct BitmexWebSocket {
///         inner: InnerWebSocket,
///     }
/// }
/// ```
#[macro_export]
macro_rules! derive_web_socket {
    (
        type Exchange = $exchange:ty;
        type Message = $message:ty;
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
        impl $crate::websocket::WebSocket for $name {
            type Exchange = $exchange;
            type Message = $message;

            fn capability() -> $crate::websocket::WebSocketCapability {
                <derive_client!(type $($field_type)*,)>::capability()
            }

            async fn authenticate(
                &mut self,
                credentials: $crate::Credentials,
            ) -> $crate::error::Result<()> {
                derive_client!(self $($field_name)*,).authenticate::<Self>(credentials).await
            }

            fn as_mut(&mut self) -> &mut $crate::websocket::WebSocketStream<$crate::websocket::MaybeTlsStream<$crate::websocket::TcpStream>> {
                &mut derive_client!(self $($field_name)*,).inner
            }
        }
    };
}

#[macro_export]
macro_rules! impl_command_converter {
    (
        for $($ws:ty),+;
        $($super_command:ty => $command:ty),+
    ) => {
        impl_command_converter!($($ws),+; $($super_command => $command),+);
    };
    ($ws:ty, $($other:ty),+; $($super_command:ty => $command:ty),+) => {
        impl_command_converter!($ws; $($super_command => $command),+);
        impl_command_converter!($($other),+; $($super_command => $command),+);
    };
    ($ws:ty; $($super_command:ty => $command:ty),+) => {
        $(
            impl CommandConverter<$super_command> for $ws {
                type Command = $command;

                fn convert_command(command: $super_command) -> Self::Command {
                    <$command>::from(command)
                }
            }
        )+
    }
}

#[macro_export]
macro_rules! impl_message_converter {
    (
        for $($ws:ty),+;
        type Msg = $message:ty;
    ) => {
        $(
            impl MessageConverter for $ws {
                type Msg = $message;

                fn convert_message(message: Self::Msg) -> SuperMessage {
                    message.into()
                }
            }

            impl WsMessage<$ws> for $message {}
        )+
    };
}
