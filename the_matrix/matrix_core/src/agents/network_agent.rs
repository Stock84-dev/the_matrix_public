pub use client::NetworkClient;
pub use exchange_state::{build_and_kill, NetworkAgentState};
pub use websocket::{run_message_loop, Ws};

// Must be at the top of a file otherwise modules that are in this module would not see this macro.
macro_rules! borrow {
    ($arg:ident, $($args:ident),+) => {
        let $arg = &$arg;
        borrow!($($args),+);
    };
    ($arg:ident) => {
        let $arg = &$arg;
    };
}

/// ```
/// broadcast_async!(self, OnMarginChanged, margin);
/// ```
/// Expands into:
/// ```
/// {
///     let margin = &margin;
///     self.listeners
///         .broadcast_async(async move |x| x.OnMarginChanged(margin).await)
///         .await?;
/// }
/// ```
macro_rules! broadcast_async {
    ($listeners:ident, $function:ident) => {{
        $listeners.listeners.broadcast_async(
            async move |x| x.$function().await
        ).await?;
    }};

    ($listeners:ident, $function:ident, $($args:ident),*) => {{
        borrow!($($args),*);
        $listeners.listeners.broadcast_async(
            async move |x| x.$function($($args),*).await
        ).await?;
    }};
}

mod client;
mod exchange_state;
mod websocket;
