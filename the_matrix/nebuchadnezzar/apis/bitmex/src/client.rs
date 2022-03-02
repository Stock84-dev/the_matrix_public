use nebuchadnezzar_core::chrono::{Duration, Utc};
use nebuchadnezzar_core::client::ring::hmac;
use nebuchadnezzar_core::client::ring::hmac::Key;
use nebuchadnezzar_core::client::{handle_response, Client, ClientCapability, Request};
use nebuchadnezzar_core::error::{AnyResult, NebError};
use nebuchadnezzar_core::log::*;
use nebuchadnezzar_core::reqwest::{ReqwestClient, StatusCode, Url};
use nebuchadnezzar_core::signatures::hmac_sha256;
use nebuchadnezzar_core::sorted_vec::SortedSet;
use nebuchadnezzar_core::tokio::sync::RwLock;
use nebuchadnezzar_core::tokio::{self};
use nebuchadnezzar_core::{async_trait, Credentials};
use serde::de::DeserializeOwned;

use crate::exchange::Bitmex;

#[derive(Debug)]
pub struct Credential {
    pub signed_key: Key,
    pub api_key: String,
}

#[derive(Debug)]
struct Limit {
    updated_ts: i64,
    remaining: u32,
    reset_ts: i64,
}

#[derive(Debug)]
pub struct BitmexClient {
    client: ReqwestClient,
    credential: Option<Credential>,
    limit: RwLock<Limit>,
    use_testnet: bool,
}

#[async_trait]
impl Client for BitmexClient {
    type Exchange = Bitmex;

    fn exchange(&self) -> Self::Exchange {
        Bitmex::new(self.use_testnet)
    }

    fn authenticate(&mut self, credentials: Credentials) -> AnyResult<()> {
        self.credential = Some(Credential {
            signed_key: hmac::Key::new(hmac::HMAC_SHA256, credentials.api_secret.as_bytes()),
            api_key: credentials.api_key,
        });
        Ok(())
    }

    fn capability() -> ClientCapability {
        use nebuchadnezzar_core::timeframes::*;
        let mut c = ClientCapability::default();
        c.timeframes = SortedSet::from(vec![m1, m5, h1, d1]);
        c
    }

    async fn request_raw<R>(
        &self,
        url: Url,
        body: String,
    ) -> AnyResult<<R as Request<Self>>::Response>
    where
        Self: Sized,
        R: Request<Self>,
        R::Response: DeserializeOwned,
    {
        // if connection breaks while in the middle of transfering data then it hangs forever
        match tokio::time::timeout(tokio::time::Duration::from_secs(20000), async {
            loop {
                trace!("request start");
                let url = url.clone();
                let body = body.clone();
                let mut builder = self
                    .client
                    .request(R::METHOD, url.clone())
                    .body(body.clone())
                    // Throws unauthorized error if we don't send this header.
                    .header("content-type", "application/json");

                if let Some(credential) = &self.credential {
                    let expires = (Utc::now() + Duration::seconds(10)).timestamp();
                    let signature =
                        hmac_sha256(&credential.signed_key, R::METHOD, expires, &url, &body);
                    builder = builder
                        .header("api-expires", expires)
                        .header("api-key", &credential.api_key)
                        .header("api-signature", signature);
                } else if R::SIGNED {
                    return Err(NebError::NoApiKeySet.into());
                }
                trace!("reading limits");
                let limit = self.limit.read().await;
                if limit.remaining == 0 {
                    let now = Utc::now().timestamp();
                    let diff = limit.reset_ts - now;
                    drop(limit);
                    if diff > 0 {
                        warn!("Rate limited for {} seconds.", diff);
                        tokio::time::sleep(tokio::time::Duration::from_secs(diff as u64)).await;
                    }
                } else {
                    drop(limit);
                }
                trace!("sending request");

                let response = builder.send().await?;
                trace!("response received");
                let headers = response.headers();
                if response.status() == StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = headers
                        .get("retry-after")
                        .unwrap()
                        .to_str()?
                        .parse::<i64>()?;
                    trace!("writing limits");
                    let mut limit = self.limit.write().await;
                    trace!("limits written");
                    limit.reset_ts = Utc::now().timestamp() + retry_after;
                    limit.remaining = 0;
                    continue;
                }
                if response.status() == StatusCode::OK {
                    let remaining = headers
                        .get("x-ratelimit-remaining")
                        .unwrap()
                        .to_str()?
                        .parse::<u32>()?;
                    let reset_ts = headers
                        .get("x-ratelimit-reset")
                        .unwrap()
                        .to_str()?
                        .parse::<i64>()?;
                    trace!("writing limits");
                    let mut limit = self.limit.write().await;
                    trace!("limits written");
                    let now = Utc::now().timestamp();
                    if limit.updated_ts < now {
                        limit.updated_ts = now;
                        limit.remaining = remaining;
                        limit.reset_ts = reset_ts;
                    }
                }
                trace!("processing response");
                let response = handle_response(response).await?;
                trace!("response processed");
                return Ok(response);
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(NebError::Timeout)?,
        }
    }
}

impl BitmexClient {
    pub(crate) fn new(use_testnet: bool) -> BitmexClient {
        BitmexClient {
            client: Default::default(),
            credential: None,
            limit: RwLock::new(Limit {
                updated_ts: i64::MIN,
                remaining: u32::MAX,
                reset_ts: i64::MIN,
            }),
            use_testnet,
        }
    }

    pub fn credential(&self) -> &Option<Credential> {
        &self.credential
    }
}
