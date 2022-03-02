use thiserror::Error;

#[derive(Error, Debug)]
pub enum FatalMatrixError {
    #[error("Max leverage has been reached.")]
    MaxLeverage,
    #[error("Order spam.")]
    MaxOrdersPerMinute,
    #[error("Something bad happened so that trading models cannot trade.")]
    Kill,
}

#[derive(Error, Debug)]
pub enum MatrixError {
    #[error("Websocket returned None.")]
    WebsocketExhausted,
    #[error("System shutdown requested.")]
    Shutdown,
    #[error("System reload requested.")]
    Reload,
}
