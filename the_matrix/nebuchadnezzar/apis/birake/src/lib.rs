#[macro_use]
extern crate serde;

pub mod definitions;
pub mod requests;

pub mod client {
    use crate::exchange::Birake;
    use nebuchadnezzar_core::client::{handle_response, Client, ClientCapability, Request};
    use nebuchadnezzar_core::error::{Error, NebError};
    use nebuchadnezzar_core::reqwest::{ReqwestClient, Url};
    use nebuchadnezzar_core::{async_trait, Credentials, Exchange};
    use serde::de::DeserializeOwned;

    #[derive(Default)]
    pub struct BirakeClient {
        client: ReqwestClient,
        credentials: Option<Credentials>,
    }

    #[async_trait]
    impl Client for BirakeClient {
        type Exchange = Birake;

        fn exchange(&self) -> Self::Exchange {
            Birake
        }

        fn authenticate(&mut self, credentials: Credentials) -> Result<(), Error> {
            self.credentials = Some(credentials);
            Ok(())
        }

        fn capability() -> ClientCapability {
            unimplemented!()
        }

        async fn request_raw<R>(
            &self,
            url: Url,
            body: String,
        ) -> Result<<R as Request<Self>>::Response, Error>
        where
            Self: Sized,
            R: Request<Self>,
            R::Response: DeserializeOwned,
        {
            let builder = if R::SIGNED {
                let credentials = self.credentials.as_ref().ok_or(NebError::NoApiKeySet)?;
                self.client
                    .request(R::METHOD, url)
                    .body(body)
                    // .header("Content-Type", "application/json")
                    .header("birake-user", &credentials.api_key)
                    .header("birake-authorization", &credentials.api_secret)
            } else {
                self.client.request(R::METHOD, url).body(body)
            };

            let response = builder.send().await?;
            handle_response(response).await
        }
    }
}

pub mod exchange {
    use crate::client::BirakeClient;
    use nebuchadnezzar_core::error::{NebError, Result};
    use nebuchadnezzar_core::sorted_vec::SortedSet;
    use nebuchadnezzar_core::websocket::{NotWebSocket, WebSocket};
    use nebuchadnezzar_core::{async_trait, Exchange};

    #[derive(Default)]
    pub struct Birake;

    #[async_trait]
    impl Exchange for Birake {
        type Client = BirakeClient;
        type WebSocket = NotWebSocket<Self>;
        const NAME: &'static str = "Birake Exchange";
        const API_VERSION: &'static str = "5.0.0";
        const SITE_URL: &'static str = "https://testnet.bitmex.com/";
        const API_URL: &'static str = "https://api.birake.com/v5";
        const WS_API_URL: &'static str = "";
        const API_DOC_URL: &'static str = "https://api.birake.com/";
        const IS_DEMO: bool = false;

        fn new_client(&self) -> Self::Client {
            BirakeClient::default()
        }

        async fn new_web_socket(&self) -> Result<Self::WebSocket> {
            NotWebSocket::unsupported()
        }
    }
}

pub mod websocket {}

#[cfg(feature = "schema")]
pub mod schema {
    use nebuchadnezzar_core::reqwest::Method;
    pub use nebuchadnezzar_core::schema::*;

    pub fn schema() -> Schema {
        Schema {
            root_url: None,
            definitions: vec![
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/public/markets/".into(),
                    method: Method::GET,
                    payload: r#""#.into(),
                    is_signed: false,
                    pre_process: PreProcess::empty(),
                },
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/public/assets/".into(),
                    method: Method::GET,
                    payload: r#""#.into(),
                    is_signed: false,
                    pre_process: PreProcess::empty(),
                },
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/public/depth/?pair=BIR_BTC".into(),
                    method: Method::GET,
                    payload: r#""#.into(),
                    is_signed: false,
                    pre_process: PreProcess::empty(),
                },
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/public/trades/?pair=BTC_USDC".into(),
                    method: Method::GET,
                    payload: r#""#.into(),
                    is_signed: false,
                    pre_process: PreProcess::empty(),
                },
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/public/ticker".into(),
                    method: Method::GET,
                    payload: r#""#.into(),
                    is_signed: false,
                    pre_process: PreProcess::empty(),
                },
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/private/balances".into(),
                    method: Method::POST,
                    payload: r#""#.into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionMethod {
                    endpoint: "https://api.birake.com/v5/private/openOrders".into(),
                    method: Method::POST,
                    payload: r#"{
                "pair": "BTC_USDC"
            }"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
            ],
            samples: vec![
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/closedOrders".into(),
                    method: Method::POST,
                    payload: r#"{
                "pair": "BTC_USDC",
                "limit": 100
            }"#
                    .into(),
                    response: r#"[
      {
          "id": "1.11.19516117",
          "order_id": "1.7.9017579",
          "pair": "BIR_BTC",
          "price": 1e-9,
          "initialAmount": 1000,
          "amount": 0,
          "side": "buy",
          "type": "limit",
          "timestamp": "2020-06-30T11:23:00",
          "status": "open"
      }
]"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/addOrder".into(),
                    method: Method::POST,
                    payload: r#"{
            "amount": 100,
            "price": 0.00000060,
            "type": "buy",
            "market": "BIR_BTC"
        }"#
                    .into(),
                    response: r#""#.into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/cancel".into(),
                    method: Method::POST,
                    payload: r#"{
    "orderId": "1.7.9017752"
}"#
                    .into(),
                    response: r#"{
    "orderId": "1.7.9017752"
}"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/deposits".into(),
                    method: Method::POST,
                    payload: r#""#.into(),
                    response: r#"[
   {
       "id": "1.11.20088036",
       "asset": "BIR",
       "amount": 50,
       "tx": "c35e49a136223a7745a446bd7d0e8783004b63960c48e0de63cd6ecd06e59f58"
   }
]"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/withdrawals".into(),
                    method: Method::POST,
                    payload: r#""#.into(),
                    response: r#"[
   {
       "id": "1.11.20087134",
       "asset": "BIR",
       "amount": 36.89192,
       "address": "KBpSJ64Gbi1mQsPvvA5MhhDpkKPNKyjFDH"
    }
]"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/withdraw".into(),
                    method: Method::POST,
                    payload: r#"{
    "asset": "BIR",
    "address": "KBpSJ64Gbi1mQsPvvA5MhhDpkKPNKyjFDH",
    "amount": 100
}"#
                    .into(),
                    response: r#"{
   "id": "1.11.20088626",
   "amount": 100
}"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
                DefinitionSample {
                    endpoint: "https://api.birake.com/v5/private/deposit".into(),
                    method: Method::POST,
                    payload: r#"{
    "asset": "BIR"
}"#
                    .into(),
                    response: r#"{
   "address": "KSaNft56gjQrkNws2d83tFjtuyjJrLhSf2"
}"#
                    .into(),
                    is_signed: true,
                    pre_process: PreProcess::empty(),
                },
            ],
            api_key: None,
            api_secret: None,
        }
    }
}
