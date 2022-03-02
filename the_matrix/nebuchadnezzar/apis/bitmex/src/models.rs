use std::convert::TryFrom;

use nebuchadnezzar_core::error::NebError;
use nebuchadnezzar_core::prelude::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Side {
    Buy,
    Sell,
    #[serde(rename = "")]
    Unknown, // BitMEX sometimes has empty side due to unknown reason
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BinSize {
    #[serde(rename = "1m")]
    Minute1,
    #[serde(rename = "5m")]
    Minute5,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "1d")]
    Day1,
}

impl TryFrom<u32> for BinSize {
    type Error = NebError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        #![allow(non_upper_case_globals)]
        use nebuchadnezzar_core::timeframes::*;
        Ok(match value {
            m1 => BinSize::Minute1,
            m5 => BinSize::Minute5,
            h1 => BinSize::Hour1,
            d1 => BinSize::Day1,
            _ => return Err(NebError::Unsupported("timeframe")),
        })
    }
}

impl Default for BinSize {
    fn default() -> Self {
        self::BinSize::Day1
    }
}

/// http://fixwiki.org/fixwiki/PegPriceType
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum PegPriceType {
    LastPeg,
    OpeningPeg,
    MidPricePeg,
    MarketPeg,
    PrimaryPeg,
    PegToVWAP,
    TrailingStopPeg,
    PegToLimitPrice,
    ShortSaleMinPricePeg,
    #[serde(rename = "")]
    Unknown, // BitMEX sometimes has empty due to unknown reason
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum OrdStatus {
    New,
    Filled,
    Canceled,
    PartiallyFilled,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum OrdType {
    Market,
    Limit,
    Stop,
    StopLimit,
    MarketIfTouched,
    LimitIfTouched,
    MarketWithLeftOverAsLimit,
    Pegged,
}

/// https://www.onixs.biz/fix-dictionary/5.0.SP2/tagNum_59.html
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum TimeInForce {
    Day,
    GoodTillCancel,
    AtTheOpening,
    ImmediateOrCancel,
    FillOrKill,
    GoodTillCrossing,
    GoodTillDate,
    AtTheClose,
    GoodThroughCrossing,
    AtCrossing,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ExecInst {
    ParticipateDoNotInitiate,
    AllOrNone,
    MarkPrice,
    IndexPrice,
    LastPrice,
    Close,
    ReduceOnly,
    Fixed,
    #[serde(rename = "")]
    Unknown, // BitMEX sometimes has empty due to unknown reason
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ExecType {
    Trade,
    Funding,
    New,
    Canceled,
    TriggeredOrActivatedBySystem,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ContingencyType {
    OneCancelsTheOther,
    OneTriggersTheOther,
    OneUpdatesTheOtherAbsolute,
    OneUpdatesTheOtherProportional,
    #[serde(rename = "")]
    Unknown, // BitMEX sometimes has empty due to unknown reason
}
