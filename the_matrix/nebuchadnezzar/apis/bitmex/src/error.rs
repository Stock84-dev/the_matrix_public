use nebuchadnezzar_core::websocket::tokio_tungstenite;

#[derive(Error, Debug)]
pub enum BitmexWsError {
    #[error("Unexpected binary message.")]
    UnexpectedBinaryMessage,
    #[error(transparent)]
    Tungstenite(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("Unexpected json message.")]
    Serde(#[from] nebuchadnezzar_core::serde_json::Error),
}
