use std::convert::Infallible;
use std::fmt::{Debug};

use crate::reqwest::header::HeaderMap;
use crate::reqwest::{StatusCode, Url};

pub type Result<T, E = NebError> = core::result::Result<T, E>;
pub type AnyResult<T> = core::result::Result<T, anyhow::Error>;
// pub type WsResult<T> = Result<T, WsError>;

#[derive(Error, Debug)]
pub enum NebError {
    #[error("No Api key set for private api")]
    NoApiKeySet,
    #[error("Error message: {0:#?}")]
    RemoteError(RemoteError),
    #[error("Unsupported: {0}")]
    Unsupported(&'static str),
    #[error("Invalid request.")]
    InvalidRequest,
    #[error("Unexpected binary message.")]
    UnexpectedBinaryMessage,
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    /// This usually happens if request has been sent and we are receving data from server but then
    /// connection suddenly breaks due to external reasons. Future will never be notified thus it
    /// hangs forever.
    #[error("Request took too long to process")]
    Timeout,
}

// #[derive(Error, Debug)]
// pub enum DynWsError {
//     #[error(transparent)]
//     Api(Box<dyn std::error::Error>),
// }

// #[derive(Error, Debug)]
// pub enum WsError<E> {
//     #[error(transparent)]
//     Api(E),
//     #[error(transparent)]
//     UrlParse(#[from] url::ParseError),
//     #[error(transparent)]
//     Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
//     #[error(transparent)]
//     Other(#[from] anyhow::Error),
// }

impl From<Infallible> for NebError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Debug)]
pub struct RemoteError {
    pub url: Url,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub text: reqwest::Result<String>,
}

impl RemoteError {
    pub async fn from(response: reqwest::Response) -> RemoteError {
        RemoteError {
            url: response.url().clone(),
            status: response.status(),
            headers: response.headers().clone(),
            text: response.text().await,
        }
    }
}

#[derive(Error, Debug)]
pub enum NotError {}
