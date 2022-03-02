use crate::prelude::{DateTime, Decimal, Utc};

#[derive(Clone, Debug)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

#[derive(Clone, Debug)]
pub struct Trade {
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub amount: Decimal,
}
