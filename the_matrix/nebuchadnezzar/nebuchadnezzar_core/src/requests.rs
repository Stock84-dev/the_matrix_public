use crate::client::SuperRequest;
use crate::definitions::{Candle, Trade};
use crate::prelude::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct CandlesGetRequest {
    pub timeframe: u32,
    pub symbol: String,
    pub count: Option<u32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}
impl SuperRequest for CandlesGetRequest {
    type SuperResponse = Vec<Candle>;
}

#[derive(Clone, Debug)]
pub struct TradesGetRequest {
    pub symbol: String,
    pub count: Option<u32>,
    pub offset: Option<i32>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}
impl SuperRequest for TradesGetRequest {
    type SuperResponse = Vec<Trade>;
}

#[derive(Clone, Debug)]
pub struct NotSuperRequest;
impl SuperRequest for NotSuperRequest {
    type SuperResponse = ();
}
