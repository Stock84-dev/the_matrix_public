use super::definitions::*;
use crate::client::BirakeClient;
use nebuchadnezzar_core::prelude::*;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GetMarkets;
impl Request<BirakeClient> for GetMarkets {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/public/markets/";
    type Response = Vec<Market>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GetAssets;
impl Request<BirakeClient> for GetAssets {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/public/assets/";
    type Response = Vec<Asset>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GetDepth {
    pub pair: String,
}
impl Request<BirakeClient> for GetDepth {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/public/depth/";
    type Response = Depth;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GetTrades {
    pub pair: String,
}
impl_request! {
    from TradesGetRequest;
    impl Request<BirakeClient> for GetTrades {
        const METHOD: Method = Method::GET;
        const SIGNED: bool = false;
        const ENDPOINT: &'static str = "/public/trades/";
        type Response = Vec<Trade>;
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GetTickers;
impl Request<BirakeClient> for GetTickers {
    const METHOD: Method = Method::GET;
    const SIGNED: bool = false;
    const ENDPOINT: &'static str = "/public/ticker";
    type Response = Vec<Ticker>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostBalances;
impl Request<BirakeClient> for PostBalances {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/balances";
    type Response = Vec<Balance>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostOpenOrders {
    pub pair: String,
}
impl Request<BirakeClient> for PostOpenOrders {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/openOrders";
    type Response = Vec<OpenOrder>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostClosedOrders {
    pub limit: Decimal,
    pub pair: String,
}
impl Request<BirakeClient> for PostClosedOrders {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/closedOrders";
    type Response = Vec<ClosedOrder>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostAddOrder {
    pub amount: Decimal,
    pub market: String,
    pub price: Decimal,
    #[serde(rename = "type")]
    pub kind: String,
}
impl Request<BirakeClient> for PostAddOrder {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/addOrder";
    type Response = AddOrder;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostCancel {
    #[serde(rename = "orderId")]
    pub order_id: String,
}
impl Request<BirakeClient> for PostCancel {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/cancel";
    type Response = bool;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostDeposits;
impl Request<BirakeClient> for PostDeposits {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/deposits";
    type Response = Vec<DepositHistory>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostWithdrawals;
impl Request<BirakeClient> for PostWithdrawals {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/withdrawals";
    type Response = Vec<Withdrawal>;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostWithdraw {
    pub address: String,
    pub amount: Decimal,
    pub asset: String,
}
impl Request<BirakeClient> for PostWithdraw {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/withdraw";
    type Response = Withdraw;
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PostDeposit {
    pub asset: String,
}
impl Request<BirakeClient> for PostDeposit {
    const METHOD: Method = Method::POST;
    const SIGNED: bool = true;
    const ENDPOINT: &'static str = "/private/deposit";
    type Response = Deposit;
}
