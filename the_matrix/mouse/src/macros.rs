mod vtable;
#[macro_export]
macro_rules! field_names {
    ($(#[$attr:meta])*
    $struct_vis:vis struct $name:ident { $($(#[$field_attr:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?}) => {
        $(#[$attr])*
        $struct_vis struct $name {
            $($(#[$field_attr])* $field_vis $field_name : $field_type),*
        }

        impl $name {
            /// Containts a list of field names.
            pub const NAMES: &'static [&'static str] = &[$(stringify!($field_name)),*];
            /// Containts a list of field types.
            pub const TYPES: &'static [&'static str] = &[$(stringify!($field_type)),*];
        }
    }
}

pub use pin_project_lite::pin_project;
pub use {futures_util, speedy, tokio, tokio_tungstenite, url};

#[macro_export]
macro_rules! impl_websocket {
    ($kind:tt, $send_msg:tt, $receive_msg:tt, $error:tt) => {
        impl $crate::macros::futures_util::sink::Sink<$send_msg> for $kind {
            type Error = $error;

            fn poll_ready(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context,
            ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
                let this = self.project();
                this.inner.poll_ready(cx).map_err(|e| e.into())
            }

            fn start_send(
                self: std::pin::Pin<&mut Self>,
                item: $send_msg,
            ) -> std::result::Result<(), Self::Error> {
                let this = self.project();
                let command = $crate::macros::speedy::Writable::write_to_vec(&item)?;
                Ok(this.inner.start_send(
                    $crate::macros::tokio_tungstenite::tungstenite::Message::Binary(command),
                )?)
            }

            fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context,
            ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
                let this = self.project();
                this.inner.poll_flush(cx).map_err(|e| e.into())
            }

            fn poll_close(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context,
            ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
                let this = self.project();
                this.inner.poll_close(cx).map_err(|e| e.into())
            }
        }

        impl $crate::macros::futures_util::stream::Stream for $kind {
            type Item = $crate::error::Result<$receive_msg>;

            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context,
            ) -> std::task::Poll<Option<Self::Item>> {
                let this = self.project();
                let poll = this.inner.poll_next(cx);
                match poll {
                    std::task::Poll::Ready(Some(Err(e))) => {
                        std::task::Poll::Ready(Some(Err(e.into())))
                    }
                    std::task::Poll::Ready(Some(Ok(m))) => match m {
                        $crate::macros::tokio_tungstenite::tungstenite::Message::Binary(
                            payload,
                        ) => match $crate::macros::speedy::Readable::read_from_buffer(&payload) {
                            Ok(m) => std::task::Poll::Ready(Some(Ok(m))),
                            Err(e) => std::task::Poll::Ready(Some(Err(e.into()))),
                        },
                        $crate::macros::tokio_tungstenite::tungstenite::Message::Close(_) => {
                            std::task::Poll::Ready(None)
                        }
                        _ => {
                            println!("Other message");
                            std::task::Poll::Pending
                        }
                    },
                    std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
                    std::task::Poll::Pending => std::task::Poll::Pending,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_basic_websocket {
    ($kind:tt, $send_msg:tt, $receive_msg:tt, $error:tt) => {
        $crate::macros::pin_project! {
            pub struct $kind {
                #[pin]
                inner: $crate::macros::tokio_tungstenite::WebSocketStream<
                    $crate::macros::tokio_tungstenite::MaybeTlsStream<
                        $crate::macros::tokio::net::TcpStream
                    >
                >,
            }
        }

        impl $kind {
            pub async fn connect(
                url: $crate::macros::url::Url,
            ) -> std::result::Result<
                (
                    $kind,
                    $crate::macros::tokio_tungstenite::tungstenite::handshake::client::Response,
                ),
                $crate::macros::tokio_tungstenite::tungstenite::Error,
            > {
                let (ws_stream, response) =
                    $crate::macros::tokio_tungstenite::connect_async(url).await?;
                Ok(($kind { inner: ws_stream }, response))
            }
        }

        impl
            From<
                $crate::macros::tokio_tungstenite::WebSocketStream<
                    $crate::macros::tokio_tungstenite::MaybeTlsStream<
                        $crate::macros::tokio::net::TcpStream,
                    >,
                >,
            > for $kind
        {
            fn from(
                ws: $crate::macros::tokio_tungstenite::WebSocketStream<
                    $crate::macros::tokio_tungstenite::MaybeTlsStream<
                        $crate::macros::tokio::net::TcpStream,
                    >,
                >,
            ) -> Self {
                Self { inner: ws }
            }
        }

        $crate::impl_websocket!($kind, $send_msg, $receive_msg, $error);
    };
}

#[macro_export]
macro_rules! ready_loop {
    ($e:expr) => {
        match $e.poll() {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! some_loop {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    };
}

#[macro_export]
macro_rules! ok_loop {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => continue,
        }
    };
}

#[macro_export]
macro_rules! ok_break {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => break,
        }
    };
}

#[macro_export]
macro_rules! some {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    };
}

#[macro_export]
macro_rules! ready {
    ($e:expr) => {
        match $e.poll() {
            Some(x) => x,
            None => return,
        }
    };
}

#[macro_export]
macro_rules! ok {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => return,
        }
    };
}

#[macro_export]
macro_rules! ok_if_err {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(_) => return Ok(()),
        }
    };
}

#[macro_export]
macro_rules! path {
    ($($path:expr),*) => {{
        let mut path = String::new();
        $(
            path.push_str($path);
            path.push('/');
        )*
        path.pop();
        path
    }}
}

// mod a {
//     use crate::error::Error;
//     use serde::{Deserialize, Serialize};
//     #[derive(Deserialize, Serialize)]
//     pub struct A {}
//     crate::impl_basic_websocket!(WebSocket, A, A, Error);
// }
