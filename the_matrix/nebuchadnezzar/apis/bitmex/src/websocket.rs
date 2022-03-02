mod command;
mod message;
mod topic;

use nebuchadnezzar_core::chrono::Duration;
use nebuchadnezzar_core::client::ring::hmac;
use nebuchadnezzar_core::error::Result;
use nebuchadnezzar_core::futures_util::stream::Fuse;
use nebuchadnezzar_core::futures_util::{FutureExt, StreamExt};
use nebuchadnezzar_core::prelude::Utc;
use nebuchadnezzar_core::reqwest::{Method, Url};
use nebuchadnezzar_core::signatures::hmac_sha256;
use nebuchadnezzar_core::tokio::net::TcpStream;
use nebuchadnezzar_core::websocket::tokio_tungstenite::tungstenite::Message as RawMessage;
use nebuchadnezzar_core::websocket::tokio_tungstenite::{
    connect_async, MaybeTlsStream, WebSocketStream,
};
use nebuchadnezzar_core::websocket::{WebSocket, WebSocketCapability, WsCommand};
use nebuchadnezzar_core::{async_trait, serde_json, Credentials, Exchange};

pub use self::command::Command;
pub use self::message::{
    Action, CancelAllAfterMessage, ErrorMessage, InfoMessage, Limit, Message, SuccessMessage,
    TableFilter, TableMessage,
};
pub use self::topic::Topic;
use crate::client::Credential;
use crate::error::BitmexWsError;
use crate::exchange::Bitmex;
use crate::nebuchadnezzar_core::futures_util::SinkExt;

pub struct BitmexWebSocket {
    inner: Fuse<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    use_testnet: bool,
}

#[async_trait]
impl WebSocket for BitmexWebSocket {
    type Error = BitmexWsError;
    type Exchange = Bitmex;
    type Message = Message;
    type RawCommand = RawMessage;

    fn exchange(&self) -> Self::Exchange {
        Bitmex::new(self.use_testnet)
    }

    fn capability() -> WebSocketCapability {
        unimplemented!()
    }

    async fn authenticate(&mut self, credentials: Credentials) -> Result<(), Self::Error> {
        let credential = Credential {
            signed_key: hmac::Key::new(hmac::HMAC_SHA256, credentials.api_secret.as_bytes()),
            api_key: credentials.api_key,
        };
        self.authenticate_raw(&credential).await
    }

    async fn send<C>(&mut self, command: C) -> Result<(), Self::Error>
    where
        Self: Sized,
        C: WsCommand<Self>,
    {
        self.inner.send(command.serialize()).await?;
        Ok(())
    }

    async fn next(&mut self) -> Option<Result<Self::Message, Self::Error>>
    where
        Self: Sized,
    {
        let mut timeouted = false;
        loop {
            let timeout = nebuchadnezzar_core::tokio::time::sleep(
                nebuchadnezzar_core::tokio::time::Duration::from_secs(5),
            )
            .fuse();
            nebuchadnezzar_core::tokio::select! {
                message = self.inner.next() => match message {
                    None => return None,
                    Some(Ok(RawMessage::Text(m))) => {
                        return Some(serde_json::from_str(&m).map_err(|x| x.into()))
                    }
                    Some(Ok(RawMessage::Binary(_))) => {
                        return Some(Err(BitmexWsError::UnexpectedBinaryMessage))
                    }
                    Some(Ok(RawMessage::Ping(_))) => {
                        if let Err(e) = self.inner.send(RawMessage::Pong("pong".into())).await {
                            return Some(Err(e.into()));
                        }
                    }
                    Some(Ok(RawMessage::Pong(_))) => {
                        if !timeouted {
                            return Some(Ok(Message::Pong));
                        }
                        timeouted = false;
                    }
                    Some(Ok(RawMessage::Close(_))) => return None,
                    Some(Err(e)) => {
                        if timeouted {
                            return None;
                        }
                        return Some(Err(e.into()));
                    }
                },
                _ = timeout => {
                    #[allow(unused_must_use)]
                    if timeouted {
                        self.inner.get_mut().close(None).await;
                        return None;
                    }
                    let result = self.inner.send(RawMessage::Ping("ping".into())).await;
                    if result.is_err() {
                        return None;
                    }
                    timeouted = true;
                }
            }
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        Ok(self.inner.get_mut().close(None).await?)
    }
}

impl BitmexWebSocket {
    pub(crate) async fn connect(use_testnet: bool) -> Result<BitmexWebSocket, BitmexWsError> {
        let url = Url::parse(Bitmex::new(use_testnet).ws_api_url()).unwrap();
        Ok(Self {
            inner: connect_async(url).await?.0.fuse(),
            use_testnet,
        })
    }

    pub async fn authenticate_raw(&mut self, credential: &Credential) -> Result<(), BitmexWsError> {
        let expires = (Utc::now() + Duration::seconds(5)).timestamp();
        let sig = hmac_sha256(
            &credential.signed_key,
            Method::GET,
            expires,
            &Url::parse(self.exchange().ws_api_url()).unwrap(),
            "",
        );
        self.send(Command::Authenticate(
            credential.api_key.clone(),
            expires,
            sig,
        ))
        .await
    }
}
