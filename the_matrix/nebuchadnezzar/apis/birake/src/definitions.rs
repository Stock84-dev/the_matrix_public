use nebuchadnezzar_core::prelude::*;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Market {
    pub base: String,
    #[serde(rename = "minPrice")]
    pub min_price: Decimal,
    #[serde(rename = "minVolume")]
    pub min_volume: Decimal,
    pub quote: String,
    pub symbol: String,
    #[serde(rename = "tickSize")]
    pub tick_size: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Asset {
    #[serde(rename = "canDeposit")]
    pub can_deposit: bool,
    #[serde(rename = "canWithdraw")]
    pub can_withdraw: bool,
    pub coininfo: String,
    #[serde(rename = "lastUpdateTimestamp")]
    pub last_update_timestamp: String,
    #[serde(rename = "makerFee")]
    pub maker_fee: Decimal,
    #[serde(rename = "maxWithdrawal")]
    pub max_withdrawal: Decimal,
    #[serde(rename = "minWithdrawal")]
    pub min_withdrawal: Decimal,
    pub name: String,
    pub precision: Decimal,
    pub symbol: String,
    #[serde(rename = "takerFee")]
    pub taker_fee: Decimal,
    #[serde(rename = "unifiedCryptoassetID")]
    pub unified_cryptoasset_id: Decimal,
    pub validator: String,
    pub wallet_enabled: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OrderBookEntry {
    pub amount: Decimal,
    #[serde(rename = "initialAmount")]
    pub initial_amount: Decimal,
    pub price: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Depth {
    pub buys: Vec<OrderBookEntry>,
    pub sells: Vec<OrderBookEntry>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Trade {
    #[serde(rename = "isBuyerMaker")]
    pub is_buyer_maker: bool,
    #[serde(rename = "marketPair")]
    pub market_pair: String,
    pub price: Decimal,
    pub time: String,
    #[serde(rename = "tradeId")]
    pub trade_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub volume: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Ticker {
    #[serde(rename = "baseVolume24h")]
    pub base_volume_2_4h: Decimal,
    #[serde(rename = "highestBid")]
    pub highest_bid: Decimal,
    #[serde(rename = "lastPrice")]
    pub last_price: Decimal,
    #[serde(rename = "lastUpdateTimestamp")]
    pub last_update_timestamp: String,
    #[serde(rename = "lowestAsk")]
    pub lowest_ask: Decimal,
    #[serde(rename = "quoteVolume24h")]
    pub quote_volume_2_4h: Decimal,
    #[serde(rename = "tradingPairs")]
    pub trading_pairs: String,
    #[serde(rename = "tradingUrl")]
    pub trading_url: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Balance {
    pub free: Decimal,
    pub name: String,
    pub total: Decimal,
    pub used: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenOrder {
    pub amount: Decimal,
    pub id: String,
    #[serde(rename = "initialAmount")]
    pub initial_amount: Decimal,
    pub pair: String,
    pub price: Decimal,
    pub side: String,
    pub status: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClosedOrder {
    pub amount: Decimal,
    pub id: String,
    #[serde(rename = "initialAmount")]
    pub initial_amount: Decimal,
    pub order_id: String,
    pub pair: String,
    pub price: Decimal,
    pub side: String,
    pub status: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AddOrder {
    pub amount: Decimal,
    pub amount_filled: Decimal,
    #[serde(rename = "type")]
    pub kind: String,
    pub open: Vec<Order>,
    pub filled: Vec<Order>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Order {
    pub id: String,
    pub price: Decimal,
    pub amount: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Cancel {
    #[serde(rename = "orderId")]
    pub order_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DepositHistory {
    pub amount: Decimal,
    pub asset: String,
    pub id: String,
    pub tx: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Withdrawal {
    pub address: String,
    pub amount: Decimal,
    pub asset: String,
    pub id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Withdraw {
    pub amount: Decimal,
    pub id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Deposit {
    pub address: String,
}
