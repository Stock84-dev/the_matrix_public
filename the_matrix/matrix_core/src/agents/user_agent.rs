use super::network_agents::*;
use crate::agents::order::Order;
use merovingian::uuid::Uuid;
use std::process::Command;

pub struct UserAgent;

impl ExchangeListener for UserAgent {
    fn on_order_executed(&mut self, order: &Order) -> Option<Vec<Uuid>> {
        Command::new("notify-send")
            .arg(format!("order executed {:#?}", order))
            .spawn()
            .unwrap();
        None
    }

    fn on_bulk_orders_placed(&mut self, orders: &Vec<Vec<Order>>, i: usize) {
        Command::new("notify-send")
            .arg(format!("order placed {:#?}", orders[i]))
            .spawn()
            .unwrap();
    }
}
