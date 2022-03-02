
use mouse::num::Decimal;
use num_enum::TryFromPrimitive;
use uuid::Uuid;

// TODO: fix Clap
#[derive(Clone, Copy, Debug, Readable, Writable, TryFromPrimitive)]
#[speedy(tag_type = u8)]
#[repr(u8)]
pub enum MaintenanceMode {
    /// Unused.
    Boot,
    /// Reload when there are no open positions and orders.
    ReloadSafe,
    /// Reload now.
    Reload,
    /// Shutdown when there are no open positions and orders.
    ShutdownSafe,
    /// Shutdown now.
    Shutdown,
    /// System crashed. Crash a system by your will.
    Crash,
    /// Websocket connection closed
    Reconnect,
}

#[derive(Clone, Debug, Writable, Readable)]
pub struct Maintenance {
    pub mode: MaintenanceMode,
    pub timestamp_s: u32,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct Announcement {
    pub link: String,
    pub title: String,
    pub content: String,
    pub timestamp_s: u32,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct Trade {
    pub timestamp_ns: u64,
    pub price: f32,
    pub amount: f32,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct ChatMessage {
    pub channel_id: u8,
    pub from_bot: u8,
    pub timestamp_ns: u64,
    pub message: String,
    pub user: String,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct Connected {
    pub bots: u32,
    pub users: u32,
    pub timestamp_s: u32,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct Funding {
    pub rate: f32,
    pub daily_rate: f32,
    pub timestamp_s: u32,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct Instrument {
    pub fair_price: f32,
    pub mark_price: f32,
    pub timestamp_ns: u64,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct Insurance {
    /// Wallet balance in satoshis for bitmex.
    pub balance: u64,
    pub timestamp_s: u32,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct PublicLiquidation {
    pub order_id: Uuid,
    /// NOTE: This field can be NaN;
    pub price: f32,
    pub amount: f32,
    pub timestamp_ns: u64,
}

#[derive(Clone, Writable, Readable, PartialEq, Debug)]
pub struct OrderBookUpdate {
    pub size: f32,
    pub price: f32,
    pub timestamp_ns: u64,
}

#[derive(Clone, Debug, Writable, Readable)]
pub struct Position {
    pub market: String,
    pub amount: Decimal,
    pub timestamp_ns: u64,
}

#[derive(Clone, Debug, Writable, Readable)]
pub struct Margin {
    pub balance: Decimal,
    pub leverage: Decimal,
    pub timestamp_ns: u64,
}
