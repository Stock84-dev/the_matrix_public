#![deny(unused_must_use)]
#![feature(macro_attributes_in_derive_output)]
#![feature(extend_one)]

pub mod client;
pub mod definitions;
pub mod error;
pub mod models;
pub mod requests;
pub mod websocket;

#[macro_use]
extern crate nebuchadnezzar_core;
#[macro_use]
extern crate thiserror;

#[macro_use]
extern crate converters;

use nebuchadnezzar_core::SuperExchange;

use crate::exchange::Bitmex;

pub fn extend_exchanges(collection: &mut impl Extend<Box<dyn SuperExchange>>) {
    collection.extend_reserve(2);
    collection.extend_one(Box::new(Bitmex::new(false)));
    collection.extend_one(Box::new(Bitmex::new(true)));
}

pub mod exchange {
    use nebuchadnezzar_core::error::Result;
    use nebuchadnezzar_core::websocket::WebSocket;
    use nebuchadnezzar_core::{async_trait, Exchange};

    use crate::client::BitmexClient;
    use crate::websocket::BitmexWebSocket;

    pub struct Bitmex {
        use_testnet: bool,
    }

    impl Bitmex {
        pub fn new(use_testnet: bool) -> Bitmex {
            Bitmex { use_testnet }
        }
    }

    #[async_trait]
    impl Exchange for Bitmex {
        type Client = BitmexClient;
        type WebSocket = BitmexWebSocket;
        fn name(&self) -> &'static str {
            match self.use_testnet {
                true => "BitMEX Testnet",
                false => "BitMEX",
            }
        }
        fn api_version(&self) -> &'static str {
            match self.use_testnet {
                true => "1.0.0",
                false => "1.0.0",
            }
        }
        fn site_url(&self) -> &'static str {
            match self.use_testnet {
                true => "https://testnet.bitmex.com/",
                false => "https://www.bitmex.com/",
            }
        }
        fn api_url(&self) -> &'static str {
            match self.use_testnet {
                true => "https://testnet.bitmex.com/api/v1",
                false => "https://www.bitmex.com/api/v1",
            }
        }
        fn ws_api_url(&self) -> &'static str {
            match self.use_testnet {
                true => "wss://testnet.bitmex.com/realtime",
                false => "wss://www.bitmex.com/realtime",
            }
        }
        fn api_doc_url(&self) -> &'static str {
            match self.use_testnet {
                true => "https://www.bitmex.com/app/apiOverview",
                false => "https://www.bitmex.com/app/apiOverview",
            }
        }
        fn is_demo(&self) -> bool {
            self.use_testnet
        }

        fn new_client(&self) -> Self::Client {
            BitmexClient::new(self.use_testnet)
        }

        async fn new_web_socket(
            &self,
        ) -> Result<Self::WebSocket, <Self::WebSocket as WebSocket>::Error> {
            BitmexWebSocket::connect(self.use_testnet).await
        }
    }
}
