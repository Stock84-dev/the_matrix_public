use converters::try_from;
use nebuchadnezzar_core::client::*;
use nebuchadnezzar_core::error::NebError;
use nebuchadnezzar_core::prelude::*;
use nebuchadnezzar_core::requests::*;

use super::definitions::*;
use crate::client::BitmexClient;
use crate::models::*;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get site announcements.
pub struct GetAnnouncementRequest {
    /// Array of column names to fetch. If omitted, will return all columns.
    pub columns: Option<Value>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get urgent (banner) announcements.
pub struct GetAnnouncementUrgentRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your API Keys.
pub struct GetApiKeyRequest {
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get chat messages.
pub struct GetChatRequest {
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting ID for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Channel id. GET /chat/channels for ids. Leave blank for all.
    #[serde(rename = "channelID")]
    pub channel_id: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Send a chat message.
pub struct PostChatRequest {
    pub message: String,
    /// Channel to post to. Default 1 (English).
    #[serde(rename = "channelID")]
    pub channel_id: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get available channels.
pub struct GetChatChannelsRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get connected users.
pub struct GetChatConnectedRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get all raw executions for your account.
pub struct GetExecutionRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get all balance-affecting executions. This includes each trade, insurance charge, and
/// settlement.
pub struct GetExecutionTradeHistoryRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get funding history.
pub struct GetFundingRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get instruments.
pub struct GetInstrumentRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
impl Pageable for GetInstrumentRequest {
    const MAX_ITEMS_PER_PAGE: u32 = 500;
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get all active instruments and instruments that have expired in <24hrs.
pub struct GetInstrumentActiveRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get all price indices.
pub struct GetInstrumentIndicesRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Helper method. Gets all active instruments and all indices. This is a join of the result of
/// /indices and /active.
pub struct GetInstrumentActiveAndIndicesRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Return all active contract series and interval pairs.
pub struct GetInstrumentActiveIntervalsRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Show constituent parts of an index.
pub struct GetInstrumentCompositeIndexRequest {
    /// The composite index symbol.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get insurance fund history.
pub struct GetInsuranceRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get current leaderboard.
pub struct GetLeaderboardRequest {
    /// Ranking type. Options: "notional", "ROE"
    pub method: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your alias on the leaderboard.
pub struct GetLeaderboardNameRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GetLeaderboardNameResponse {
    pub name: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get liquidation orders.
pub struct GetLiquidationRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your current GlobalNotifications.
pub struct GetGlobalNotificationRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your orders.
pub struct GetOrderRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Create a new order.
pub struct PostOrderRequest {
    /// Instrument symbol. e.g. 'XBTUSD'.
    pub symbol: String,
    /// Order side. Valid options: Buy, Sell. Defaults to 'Buy' unless `orderQty` is negative.
    pub side: Option<Side>,
    /// Deprecated: simple orders are not supported after 2018/10/26
    #[serde(rename = "simpleOrderQty")]
    pub simple_order_qty: Option<Decimal>,
    /// Order quantity in units of the instrument (i.e. contracts).
    #[serde(rename = "orderQty")]
    pub order_qty: Option<i32>,
    /// Optional limit price for 'Limit', 'StopLimit', and 'LimitIfTouched' orders.
    pub price: Option<Decimal>,
    /// Optional quantity to display in the book. Use 0 for a fully hidden order.
    #[serde(rename = "displayQty")]
    pub display_qty: Option<i32>,
    /// Optional trigger price for 'Stop', 'StopLimit', 'MarketIfTouched', and 'LimitIfTouched'
    /// orders. Use a price below the current price for stop-sell orders and buy-if-touched orders.
    /// Use `execInst` of 'MarkPrice' or 'LastPrice' to define the current price used for
    /// triggering.
    #[serde(rename = "stopPx")]
    pub stop_px: Option<Decimal>,
    /// Optional Client Order ID. This clOrdID will come back on the order and any related
    /// executions.
    #[serde(rename = "clOrdID")]
    pub cl_ord_id: Option<String>,
    /// Deprecated: linked orders are not supported after 2018/11/10.
    #[serde(rename = "clOrdLinkID")]
    pub cl_ord_link_id: Option<String>,
    /// Optional trailing offset from the current price for 'Stop', 'StopLimit', 'MarketIfTouched',
    /// and 'LimitIfTouched' orders; use a negative offset for stop-sell orders and buy-if-touched
    /// orders. Optional offset from the peg price for 'Pegged' orders.
    #[serde(rename = "pegOffsetValue")]
    pub peg_offset_value: Option<Decimal>,
    /// Optional peg price type. Valid options: LastPeg, MidPricePeg, MarketPeg, PrimaryPeg,
    /// TrailingStopPeg.
    #[serde(rename = "pegPriceType")]
    pub peg_price_type: Option<PegPriceType>,
    /// Order type. Valid options: Market, Limit, Stop, StopLimit, MarketIfTouched, LimitIfTouched,
    /// Pegged. Defaults to 'Limit' when `price` is specified. Defaults to 'Stop' when `stopPx` is
    /// specified. Defaults to 'StopLimit' when `price` and `stopPx` are specified.
    #[serde(rename = "ordType")]
    pub ord_type: Option<OrdType>,
    /// Time in force. Valid options: Day, GoodTillCancel, ImmediateOrCancel, FillOrKill. Defaults
    /// to 'GoodTillCancel' for 'Limit', 'StopLimit', and 'LimitIfTouched' orders.
    #[serde(rename = "timeInForce")]
    pub time_in_force: Option<TimeInForce>,
    /// Optional execution instructions. Valid options: ParticipateDoNotInitiate, AllOrNone,
    /// MarkPrice, IndexPrice, LastPrice, Close, ReduceOnly, Fixed. 'AllOrNone' instruction
    /// requires `displayQty` to be 0. 'MarkPrice', 'IndexPrice' or 'LastPrice' instruction valid
    /// for 'Stop', 'StopLimit', 'MarketIfTouched', and 'LimitIfTouched' orders.
    #[serde(rename = "execInst")]
    pub exec_inst: Option<ExecInst>,
    /// Deprecated: linked orders are not supported after 2018/11/10.
    #[serde(rename = "contingencyType")]
    pub contingency_type: Option<ContingencyType>,
    /// Optional order annotation. e.g. 'Take profit'.
    pub text: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Amend the quantity or price of an open order.
pub struct PutOrderRequest {
    /// Order ID
    #[serde(rename = "orderID")]
    pub order_id: Option<String>,
    /// Client Order ID. See POST /order.
    #[serde(rename = "origClOrdID")]
    pub orig_cl_ord_id: Option<String>,
    /// Optional new Client Order ID, requires `origClOrdID`.
    #[serde(rename = "clOrdID")]
    pub cl_ord_id: Option<String>,
    /// Deprecated: simple orders are not supported after 2018/10/26
    #[serde(rename = "simpleOrderQty")]
    pub simple_order_qty: Option<Decimal>,
    /// Optional order quantity in units of the instrument (i.e. contracts).
    #[serde(rename = "orderQty")]
    pub order_qty: Option<i32>,
    /// Deprecated: simple orders are not supported after 2018/10/26
    #[serde(rename = "simpleLeavesQty")]
    pub simple_leaves_qty: Option<Decimal>,
    /// Optional leaves quantity in units of the instrument (i.e. contracts). Useful for amending
    /// partially filled orders.
    #[serde(rename = "leavesQty")]
    pub leaves_qty: Option<i32>,
    /// Optional limit price for 'Limit', 'StopLimit', and 'LimitIfTouched' orders.
    pub price: Option<Decimal>,
    /// Optional trigger price for 'Stop', 'StopLimit', 'MarketIfTouched', and 'LimitIfTouched'
    /// orders. Use a price below the current price for stop-sell orders and buy-if-touched orders.
    #[serde(rename = "stopPx")]
    pub stop_px: Option<Decimal>,
    /// Optional trailing offset from the current price for 'Stop', 'StopLimit', 'MarketIfTouched',
    /// and 'LimitIfTouched' orders; use a negative offset for stop-sell orders and buy-if-touched
    /// orders. Optional offset from the peg price for 'Pegged' orders.
    #[serde(rename = "pegOffsetValue")]
    pub peg_offset_value: Option<Decimal>,
    /// Optional amend annotation. e.g. 'Adjust skew'.
    pub text: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Cancel order(s). Send multiple order IDs to cancel in bulk.
pub struct DeleteOrderRequest {
    /// Order ID(s).
    #[serde(rename = "orderID")]
    pub order_id: Option<Value>,
    /// Client Order ID(s). See POST /order.
    #[serde(rename = "clOrdID")]
    pub cl_ord_id: Option<Value>,
    /// Optional cancellation annotation. e.g. 'Spread Exceeded'.
    pub text: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Create multiple new orders for the same symbol.
pub struct PostOrderBulkRequest {
    /// An array of orders.
    pub orders: Option<Vec<PostOrderRequest>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Amend multiple orders for the same symbol.
pub struct PutOrderBulkRequest {
    /// An array of orders.
    pub orders: Option<Vec<PutOrderRequest>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Close a position. [Deprecated, use POST /order with execInst: 'Close']
pub struct PostOrderClosePositionRequest {
    /// Symbol of position to close.
    pub symbol: String,
    /// Optional limit price.
    pub price: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Cancels all of your orders.
pub struct DeleteOrderAllRequest {
    /// Optional symbol. If provided, only cancels orders for that symbol.
    pub symbol: Option<String>,
    /// Optional filter for cancellation. Use to only cancel some orders, e.g. `{"side": "Buy"}`.
    pub filter: Option<Value>,
    /// Optional cancellation annotation. e.g. 'Spread Exceeded'
    pub text: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Automatically cancel all your orders after a specified timeout.
pub struct PostOrderCancelAllAfterRequest {
    /// Timeout in ms. Set to 0 to cancel this timer.
    pub timeout: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PostOrderCancelAllAfterResponse(Value);
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Get current orderbook in vertical format.
pub struct GetOrderBookL2Request {
    /// Instrument symbol. Send a series (e.g. XBT) to get data for the nearest contract in that
    /// series.
    pub symbol: String,
    /// Orderbook depth per side. Send 0 for full depth.
    pub depth: Option<i32>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your positions.
pub struct GetPositionRequest {
    /// Table filter. For example, send {"symbol": "XBTUSD"}.
    pub filter: Option<Value>,
    /// Which columns to fetch. For example, send ["columnName"].
    pub columns: Option<Value>,
    /// Number of rows to fetch.
    pub count: Option<i32>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Enable isolated margin or cross margin per-position.
pub struct PostPositionIsolateRequest {
    /// Position symbol to isolate.
    pub symbol: String,
    /// True for isolated margin, false for cross margin.
    pub enabled: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Update your risk limit.
pub struct PostPositionRiskLimitRequest {
    /// Symbol of position to update risk limit on.
    pub symbol: String,
    /// New Risk Limit, in Satoshis.
    #[serde(rename = "riskLimit")]
    pub risk_limit: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Transfer equity in or out of a position.
pub struct PostPositionTransferMarginRequest {
    /// Symbol of position to isolate.
    pub symbol: String,
    /// Amount to transfer, in Satoshis. May be negative.
    pub amount: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Choose leverage for a position.
pub struct PostPositionLeverageRequest {
    /// Symbol of position to adjust.
    pub symbol: String,
    /// Leverage value. Send a number between 0.01 and 100 to enable isolated margin with a fixed
    /// leverage. Send 0 to enable cross margin.
    pub leverage: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get Quotes.
pub struct GetQuoteRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get previous quotes in time buckets.
pub struct GetQuoteBucketedRequest {
    /// Time interval to bucket by. Available options: [1m,5m,1h,1d].
    #[serde(rename = "binSize")]
    pub bin_size: Option<BinSize>,
    /// If true, will send in-progress (incomplete) bins for the current time period.
    pub partial: Option<bool>,
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get model schemata for data objects returned by this API.
pub struct GetSchemaRequest {
    /// Optional model filter. If omitted, will return all models.
    pub model: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GetSchemaResponse(Value);
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Returns help text & subject list for websocket usage.
pub struct GetSchemaWebsocketHelpRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GetSchemaWebsocketHelpResponse(Value);
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get settlement history.
pub struct GetSettlementRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    pub symbol: Option<String>,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    pub count: Option<i32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get exchange-wide and per-series turnover and volume statistics.
pub struct GetStatsRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get historical exchange-wide and per-series turnover and volume statistics.
pub struct GetStatsHistoryRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get a summary of exchange statistics in USD.
pub struct GetStatsHistoryUSDRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get Trades.
#[try_from(TradesGetRequest, skipping, error = "NebError")]
pub struct GetTradesRequest {
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    #[try_from(include)]
    pub symbol: String,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    #[try_from(include)]
    pub count: Option<u32>,
    /// Starting point for results.
    #[try_from(rename = "offset")]
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    #[try_from(include)]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    #[try_from(include)]
    pub end_time: Option<DateTime<Utc>>,
}
impl Pageable for GetTradesRequest {
    const MAX_ITEMS_PER_PAGE: u32 = 1000;
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get previous trades in time buckets.
#[try_from(CandlesGetRequest, skipping, error = "NebError")]
pub struct GetTradeBucketedRequest {
    /// Time interval to bucket by. Available options: [1m,5m,1h,1d].
    #[serde(rename = "binSize")]
    #[try_from(rename = "timeframe")]
    pub bin_size: BinSize,
    /// If true, will send in-progress (incomplete) bins for the current time period.
    pub partial: Option<bool>,
    /// Instrument symbol. Send a bare series (e.g. XBT) to get data for the nearest expiring
    /// contract in that series.  You can also send a timeframe, e.g. `XBT:quarterly`. Timeframes
    /// are `nearest`, `daily`, `weekly`, `monthly`, `quarterly`, `biquarterly`, and `perpetual`.
    #[try_from(include)]
    pub symbol: String,
    /// Generic table filter. Send JSON key/value pairs, such as `{"key": "value"}`. You can key on individual fields, and do more advanced querying on timestamps. See the [Timestamp Docs](https://www.bitmex.com/app/restAPI#Timestamp-Filters) for more details.
    pub filter: Option<Value>,
    /// Array of column names to fetch. If omitted, will return all columns.  Note that this method
    /// will always return item keys, even when not specified, so you may receive more columns that
    /// you expect.
    pub columns: Option<Value>,
    /// Number of results to fetch.
    #[try_from(include)]
    pub count: Option<u32>,
    /// Starting point for results.
    pub start: Option<i32>,
    /// If true, will sort results newest first.
    pub reverse: Option<bool>,
    /// Starting date filter for results.
    #[serde(rename = "startTime")]
    #[try_from(include)]
    pub start_time: Option<DateTime<Utc>>,
    /// Ending date filter for results.
    #[serde(rename = "endTime")]
    #[try_from(include)]
    pub end_time: Option<DateTime<Utc>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get a deposit address.
pub struct GetUserDepositAddressRequest {
    pub currency: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your current wallet information.
pub struct GetUserWalletRequest {
    pub currency: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get a history of all of your wallet transactions (deposits, withdrawals, PNL).
pub struct GetUserWalletHistoryRequest {
    pub currency: Option<String>,
    /// Number of results to fetch.
    pub count: Option<f64>,
    /// Starting point for results.
    pub start: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get a summary of all of your wallet transactions (deposits, withdrawals, PNL).
pub struct GetUserWalletSummaryRequest {
    pub currency: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Get the execution history by day.
pub struct GetUserExecutionHistoryRequest {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get the minimum withdrawal fee for a currency.
pub struct GetUserMinWithdrawalFeeRequest {
    pub currency: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GetUserMinWithdrawalFeeResponse(Value);
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Request<Client> a withdrawal to an external wallet.
pub struct PostUserRequestWithdrawalRequest {
    /// 2FA token. Required if 2FA is enabled on your account.
    #[serde(rename = "otpToken")]
    pub otp_token: Option<String>,
    /// Currency you're withdrawing. Options: `XBt`
    pub currency: String,
    /// Amount of withdrawal currency.
    pub amount: i64,
    /// Destination Address.
    pub address: String,
    /// Network fee for Bitcoin withdrawals. If not specified, a default value will be calculated
    /// based on Bitcoin network conditions. You will have a chance to confirm this via email.
    pub fee: Option<f64>,
    /// Optional annotation, e.g. 'Transfer to home wallet'.
    pub text: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Cancel a withdrawal.
pub struct PostUserCancelWithdrawalRequest {
    pub token: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Confirm a withdrawal.
pub struct PostUserConfirmWithdrawalRequest {
    pub token: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Confirm your email address with a token.
pub struct PostUserConfirmEmailRequest {
    pub token: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your current affiliate/referral status.
pub struct GetUserAffiliateStatusRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Check if a referral code is valid.
pub struct GetUserCheckReferralCodeRequest {
    #[serde(rename = "referralCode")]
    pub referral_code: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get 7 days worth of Quote Fill Ratio statistics.
pub struct GetUserQuoteFillRatioRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Log out of BitMEX.
pub struct PostUserLogoutRequest;
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Save user preferences.
pub struct PostUserPreferencesRequest {
    pub prefs: Value,
    /// If true, will overwrite all existing preferences.
    pub overwrite: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your user model.
pub struct GetUserRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your account's commission status.
pub struct GetUserCommissionRequest;
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your account's margin status. Send a currency of "all" to receive an array of all supported
/// currencies.
pub struct GetUserMarginRequest {
    pub currency: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
/// Register your communication token for mobile clients
pub struct PostUserCommunicationTokenRequest {
    pub token: String,
    #[serde(rename = "platformAgent")]
    pub platform_agent: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
/// Get your user events
pub struct GetUserEventRequest {
    /// Number of results to fetch.
    pub count: Option<f64>,
    /// Cursor for pagination.
    #[serde(rename = "startId")]
    pub start_id: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct GetUserEventResponse {
    #[serde(rename = "userEvents")]
    pub user_events: Vec<UserEvent>,
}
impl Request<BitmexClient> for GetAnnouncementRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/announcement";
    type Response = Vec<Announcement>;
}
impl Request<BitmexClient> for GetAnnouncementUrgentRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/announcement/urgent";
    type Response = Vec<Announcement>;
}
impl Request<BitmexClient> for GetApiKeyRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/apiKey";
    type Response = Vec<APIKey>;
}
impl Request<BitmexClient> for GetChatRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/chat";
    type Response = Vec<Chat>;
}
impl Request<BitmexClient> for PostChatRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/chat";
    type Response = Chat;
}
impl Request<BitmexClient> for GetChatChannelsRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/chat/channels";
    type Response = Vec<ChatChannel>;
}
impl Request<BitmexClient> for GetChatConnectedRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/chat/connected";
    type Response = ConnectedUsers;
}
impl Request<BitmexClient> for GetExecutionRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/execution";
    type Response = Vec<Execution>;
}
impl Request<BitmexClient> for GetExecutionTradeHistoryRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/execution/tradeHistory";
    type Response = Vec<Execution>;
}
impl Request<BitmexClient> for GetFundingRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/funding";
    type Response = Vec<Funding>;
}
impl Request<BitmexClient> for GetInstrumentRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/instrument";
    type Response = Vec<Instrument>;
}
impl Request<BitmexClient> for GetInstrumentActiveRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/instrument/active";
    type Response = Vec<Instrument>;
}
impl Request<BitmexClient> for GetInstrumentIndicesRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/instrument/indices";
    type Response = Vec<Instrument>;
}
impl Request<BitmexClient> for GetInstrumentActiveAndIndicesRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/instrument/activeAndIndices";
    type Response = Vec<Instrument>;
}
impl Request<BitmexClient> for GetInstrumentActiveIntervalsRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/instrument/activeIntervals";
    type Response = InstrumentInterval;
}
impl Request<BitmexClient> for GetInstrumentCompositeIndexRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/instrument/compositeIndex";
    type Response = Vec<IndexComposite>;
}
impl Request<BitmexClient> for GetInsuranceRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/insurance";
    type Response = Vec<Insurance>;
}
impl Request<BitmexClient> for GetLeaderboardRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/leaderboard";
    type Response = Vec<Leaderboard>;
}
impl Request<BitmexClient> for GetLeaderboardNameRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/leaderboard/name";
    type Response = GetLeaderboardNameResponse;
}
impl Request<BitmexClient> for GetLiquidationRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/liquidation";
    type Response = Vec<Liquidation>;
}
impl Request<BitmexClient> for GetGlobalNotificationRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/globalNotification";
    type Response = Vec<GlobalNotification>;
}
impl Request<BitmexClient> for GetOrderRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order";
    type Response = Vec<Order>;
}
impl Request<BitmexClient> for PostOrderRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order";
    type Response = Order;
}
impl Request<BitmexClient> for PutOrderRequest {
    const METHOD: Method = Method::PUT;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order";
    type Response = Order;
}
impl Request<BitmexClient> for DeleteOrderRequest {
    const METHOD: Method = Method::DELETE;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order";
    type Response = Vec<Order>;
}
impl Request<BitmexClient> for PostOrderBulkRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order/bulk";
    type Response = Vec<Order>;
}
impl Request<BitmexClient> for PutOrderBulkRequest {
    const METHOD: Method = Method::PUT;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order/bulk";
    type Response = Vec<Order>;
}
impl Request<BitmexClient> for PostOrderClosePositionRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order/closePosition";
    type Response = Order;
}
impl Request<BitmexClient> for DeleteOrderAllRequest {
    const METHOD: Method = Method::DELETE;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order/all";
    type Response = Vec<Order>;
}
impl Request<BitmexClient> for PostOrderCancelAllAfterRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/order/cancelAllAfter";
    type Response = PostOrderCancelAllAfterResponse;
}
impl Request<BitmexClient> for GetOrderBookL2Request {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/orderBook/L2";
    type Response = Vec<OrderBookL2>;
}
impl Request<BitmexClient> for GetPositionRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/position";
    type Response = Vec<Position>;
}
impl Request<BitmexClient> for PostPositionIsolateRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/position/isolate";
    type Response = Position;
}
impl Request<BitmexClient> for PostPositionRiskLimitRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/position/riskLimit";
    type Response = Position;
}
impl Request<BitmexClient> for PostPositionTransferMarginRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/position/transferMargin";
    type Response = Position;
}
impl Request<BitmexClient> for PostPositionLeverageRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/position/leverage";
    type Response = Position;
}
impl Request<BitmexClient> for GetQuoteRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/quote";
    type Response = Vec<Quote>;
}
impl Request<BitmexClient> for GetQuoteBucketedRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/quote/bucketed";
    type Response = Vec<Quote>;
}
impl Request<BitmexClient> for GetSchemaRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/schema";
    type Response = GetSchemaResponse;
}
impl Request<BitmexClient> for GetSchemaWebsocketHelpRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/schema/websocketHelp";
    type Response = GetSchemaWebsocketHelpResponse;
}
impl Request<BitmexClient> for GetSettlementRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/settlement";
    type Response = Vec<Settlement>;
}
impl Request<BitmexClient> for GetStatsRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/stats";
    type Response = Vec<Stats>;
}
impl Request<BitmexClient> for GetStatsHistoryRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/stats/history";
    type Response = Vec<StatsHistory>;
}
impl Request<BitmexClient> for GetStatsHistoryUSDRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/stats/historyUSD";
    type Response = Vec<StatsUSD>;
}
converter! {
    from TradesGetRequest;
    impl Request<BitmexClient> for GetTradesRequest {
        const METHOD: Method = Method::GET;
        const SIGNED: bool = false;
        const ENDPOINT: &'static str = "/trade";
        type Response = Vec<Trade>;
    }
}
converter! {
    from nebuchadnezzar_core::requests::CandlesGetRequest;
    impl Request<BitmexClient> for GetTradeBucketedRequest {
        const METHOD: Method = Method::GET;
        const SIGNED: bool = false;
        const ENDPOINT: &'static str = "/trade/bucketed";
        type Response = Vec<TradeBin>;
    }
}
impl Request<BitmexClient> for GetUserDepositAddressRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/depositAddress";
    type Response = String;
}
impl Request<BitmexClient> for GetUserWalletRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/wallet";
    type Response = Wallet;
}
impl Request<BitmexClient> for GetUserWalletHistoryRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/walletHistory";
    type Response = Vec<Transaction>;
}
impl Request<BitmexClient> for GetUserWalletSummaryRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/walletSummary";
    type Response = Vec<Transaction>;
}
impl Request<BitmexClient> for GetUserExecutionHistoryRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/executionHistory";
    type Response = Vec<ExecutionHistory>;
}
impl Request<BitmexClient> for GetUserMinWithdrawalFeeRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/user/minWithdrawalFee";
    type Response = GetUserMinWithdrawalFeeResponse;
}
impl Request<BitmexClient> for PostUserRequestWithdrawalRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/requestWithdrawal";
    type Response = Transaction;
}
impl Request<BitmexClient> for PostUserCancelWithdrawalRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/user/cancelWithdrawal";
    type Response = Transaction;
}
impl Request<BitmexClient> for PostUserConfirmWithdrawalRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/user/confirmWithdrawal";
    type Response = Transaction;
}
impl Request<BitmexClient> for PostUserConfirmEmailRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/user/confirmEmail";
    type Response = AccessToken;
}
impl Request<BitmexClient> for GetUserAffiliateStatusRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/affiliateStatus";
    type Response = Affiliate;
}
impl Request<BitmexClient> for GetUserCheckReferralCodeRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/user/checkReferralCode";
    type Response = f64;
}
impl Request<BitmexClient> for GetUserQuoteFillRatioRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/quoteFillRatio";
    type Response = QuoteFillRatio;
}
impl Request<BitmexClient> for PostUserLogoutRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/user/logout";
    type Response = ();
}
impl Request<BitmexClient> for PostUserPreferencesRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/preferences";
    type Response = User;
}
impl Request<BitmexClient> for GetUserRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user";
    type Response = User;
}
impl Request<BitmexClient> for GetUserCommissionRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/commission";
    type Response = UserCommissionsBySymbol;
}
impl Request<BitmexClient> for GetUserMarginRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/margin";
    type Response = Margin;
}
impl Request<BitmexClient> for PostUserCommunicationTokenRequest {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/user/communicationToken";
    type Response = Vec<CommunicationToken>;
}
impl Request<BitmexClient> for GetUserEventRequest {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/userEvent";
    type Response = GetUserEventResponse;
}
impl Pageable for GetTradeBucketedRequest {
    const MAX_ITEMS_PER_PAGE: u32 = 1000;
}
