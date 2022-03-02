pub use nebuchadnezzar_core as core;
use nebuchadnezzar_core::SuperExchange;

macro_rules! include_all {
    ($($modules:ident),+) => {
        pub mod implementations {
            $(pub use $modules;)+
        }
        pub mod exchanges {
            $(pub use $modules::exchange::*;)+
        }
        pub mod clients {
            $(pub use $modules::client::*;)+
        }
        pub mod websockets {
            $(pub use $modules::websocket::*;)+
        }
        pub fn exchanges() -> Vec<Box<dyn SuperExchange>> {
            let mut exchanges: Vec<Box<dyn SuperExchange>> = vec![];
            $($modules::extend_exchanges(&mut exchanges);)+
            exchanges
        }
    }
}

include_all!(bitmex);
