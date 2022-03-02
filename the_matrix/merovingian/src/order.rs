use std::collections::HashMap;
use std::sync::Mutex;

use base64::STANDARD_NO_PAD;
use lazy_static::lazy_static;
use mouse::error::Result;
use mouse::num::traits::{FromPrimitive, ToPrimitive, Zero};
use mouse::num::{Decimal, FromMaybeDecimal, IntoDecimal};

#[derive(Clone, Debug, Readable, Writable)]
pub struct Order {
    /// How much to buy/sell.
    /// If order is for inverse market amount is USD otherwise XBT for XBTUSD market.
    /// Can be negative, indicating short position.
    pub amount: Decimal,
    pub trigger_price: Option<Decimal>,
    pub limit: Option<Decimal>,
    pub executed_price: Option<Decimal>,
    pub market: String,
    pub id: OrderId,
    pub predicted_price: f32,
    /// Overall value of the order.
    /// If order is for inverse market then value is in XBT otherwise USD for XBTUSD market.
    /// Can be negative, indicating short position.
    pub value: Option<Decimal>,
    //    pub link: Uuid,
    pub timestamp_ns: u64,
}

impl Order {
    pub fn new(
        model_id: u32,
        open_cl_order: &OpenCLOrder,
        market: &String,
        last_price: f32,
        inverse: bool,
    ) -> Order {
        let predicted_price = if open_cl_order.amount.is_nan() {
            f32::NAN
        } else if !open_cl_order.limit.is_nan() {
            open_cl_order.limit
        } else if !open_cl_order.trigger_price.is_nan() {
            open_cl_order.trigger_price
        } else {
            last_price
        };
        let amount = Decimal::from_f32(open_cl_order.amount).unwrap_or(Decimal::zero());
        let value = if predicted_price.is_nan() {
            None
        } else {
            Some(value(
                Decimal::from_f32(predicted_price).unwrap(),
                amount,
                inverse,
            ))
        };
        Order {
            amount,
            trigger_price: open_cl_order.trigger_price.to_decimal(),
            limit: open_cl_order.limit.to_decimal(),
            executed_price: open_cl_order.executed_price.to_decimal(),
            market: market.clone(),
            id: IdGenerator::new_order_id(model_id),
            predicted_price,
            value,
            timestamp_ns: 0,
        }
    }

    pub fn from_market_orders(orders: &[Order], inverse: bool) -> Option<Order> {
        let amount: Decimal = orders.iter().map(|x| x.amount).sum();
        if amount.is_zero() {
            return None;
        }
        let predicted_price = orders[0].predicted_price;
        Some(Order {
            amount,
            trigger_price: None,
            limit: None,
            executed_price: None,
            market: orders[0].market.clone(),
            id: orders[0].id,
            predicted_price,
            value: Some(value(
                Decimal::from_f32(predicted_price).unwrap(),
                amount,
                inverse,
            )),
            timestamp_ns: orders[0].timestamp_ns,
        })
    }

    /// Compares by amount, value, trigger_price and limit
    pub fn is_partially_equal(&self, order: &Order) -> bool {
        self.amount == order.amount
            && self.value == order.value
            && self.trigger_price == order.trigger_price
            && self.limit == order.limit
    }

    pub fn change_amount(&mut self, executed_price: Decimal, inverse: bool) {
        if inverse {
            self.amount = self.value.unwrap() * executed_price;
        } else {
            self.amount = self.value.unwrap() / executed_price;
        }
    }

    pub fn cancel(&mut self) {
        self.amount = Decimal::zero();
    }

    pub fn is_canceled(&self) -> bool {
        self.amount.is_zero()
    }

    pub fn is_market(&self) -> bool {
        !self.amount.is_zero()
            && self.trigger_price.is_none()
            && self.limit.is_none()
            && self.executed_price.is_none()
    }

    pub fn is_stop_market(&self) -> bool {
        !self.amount.is_zero()
            && !self.trigger_price.is_none()
            && self.limit.is_none()
            && self.executed_price.is_none()
    }
}

pub fn value(price: Decimal, amount: Decimal, inverse: bool) -> Decimal {
    if inverse {
        amount / price
    } else {
        amount * price
    }
}

pub fn executed_price(amount: Decimal, value: Decimal, inverse: bool) -> Decimal {
    if inverse {
        amount / value
    } else {
        value / amount
    }
}

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct OpenCLOrder {
    pub amount: f32,
    pub trigger_price: f32,
    pub limit: f32,
    pub executed_price: f32,
}

impl OpenCLOrder {
    pub fn new() -> OpenCLOrder {
        OpenCLOrder {
            amount: f32::NAN,
            trigger_price: f32::NAN,
            limit: f32::NAN,
            executed_price: f32::NAN,
        }
    }
}
impl From<&Order> for OpenCLOrder {
    fn from(order: &Order) -> OpenCLOrder {
        OpenCLOrder {
            amount: order.amount.to_f32().unwrap(),
            trigger_price: order.trigger_price.to_f32(),
            limit: order.limit.to_f32(),
            executed_price: order.executed_price.to_f32(),
        }
    }
}

pub struct IdGenerator;

lazy_static! {
    static ref ORDER_INDEX: Mutex<HashMap<u32, u32>> = Default::default();
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Readable, Writable)]
pub struct OrderId {
    model_id: u32,
    i: u32,
}

impl OrderId {
    pub fn unknown() -> OrderId {
        OrderId {
            model_id: u32::MAX,
            i: u32::MAX,
        }
    }

    pub fn is_known(&self) -> bool {
        self.model_id != u32::MAX && self.i != u32::MAX
    }

    pub fn from_str(s: &str) -> Self {
        let mut iter = s.split('-');
        let mut buf = [0u8; 4];
        let mut next;
        let result: Result<_, base64::DecodeError> = try {
            if let Some(n) = iter.next() {
                next = n;
            } else {
                return OrderId::unknown();
            }
            base64::decode_config_slice(next, STANDARD_NO_PAD, &mut buf)?;
            let model_id = u32::from_be_bytes(buf);
            if let Some(n) = iter.next() {
                next = n;
            } else {
                return OrderId::unknown();
            }
            base64::decode_config_slice(next, STANDARD_NO_PAD, &mut buf)?;
            let i = u32::from_be_bytes(buf);
            OrderId { model_id, i }
        };
        match result {
            Ok(result) => result,
            Err(_) => OrderId::unknown(),
        }
    }
}

impl ToString for OrderId {
    fn to_string(&self) -> String {
        let mut id = String::with_capacity(2 * 6 + 1);
        base64::encode_config_buf(self.model_id.to_be_bytes(), STANDARD_NO_PAD, &mut id);
        id.push('-');
        base64::encode_config_buf(self.i.to_be_bytes(), STANDARD_NO_PAD, &mut id);
        id
    }
}

impl IdGenerator {
    pub fn new_order_id(model_id: u32) -> OrderId {
        let mut guard = ORDER_INDEX.lock().unwrap();
        let i;
        if let Some(id) = guard.get_mut(&model_id) {
            *id += 1;
            i = *id;
        } else {
            guard.insert(model_id, 0);
            i = 0;
        }
        OrderId { model_id, i }
    }
}
