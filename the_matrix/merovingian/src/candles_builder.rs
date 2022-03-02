use std::collections::HashMap;

use mouse::log::*;
use mouse::num::NumExt;

use crate::candles::{Candle, Candles};


pub struct CandlesBuilder {
    candles: HashMap<String, HashMap<u32, Candles>>,
}

impl CandlesBuilder {
    pub fn new() -> CandlesBuilder {
        CandlesBuilder {
            candles: Default::default(),
        }
    }

    pub fn candles(&self) -> &HashMap<String, HashMap<u32, Candles>> {
        &self.candles
    }

    pub fn min_timeframe(&self) -> u32 {
        let mut min_timeframe = u32::MAX;
        for (_, map) in &self.candles {
            for (timeframe, _) in map {
                min_timeframe = min_timeframe.min(*timeframe);
            }
        }
        min_timeframe
    }

    pub fn insert(&mut self, market: &String, candles: Candles) {
        match self.candles.get_mut(market) {
            None => {
                let mut map = HashMap::new();
                map.insert(candles.timeframe_step(), candles);
                self.candles.insert(market.to_string(), map);
            }
            Some(map) => {
                map.insert(candles.timeframe_step(), candles);
            }
        }
    }

    pub fn override_candle(&mut self, market: &String, candle: &Candle, timeframe: u32) {
        let candles = self
            .candles
            .get_mut(market)
            .unwrap()
            .get_mut(&timeframe)
            .unwrap();
        let id = ((candle.timestamp - candles.timestamp[0]) / timeframe) as usize;
        candles.open[id] = candle.open;
        candles.high[id] = candle.high;
        candles.low[id] = candle.low;
        candles.close[id] = candle.close;
        candles.volume[id] = candle.volume;
        candles.timestamp[id] = candle.timestamp;
    }

    pub fn tick_empty_all_markets(&mut self, timestamp_s: u32) -> u32 {
        let mut max_candle_timestamp = 0;
        for (_market, timeframe_map) in &mut self.candles {
            max_candle_timestamp =
                max_candle_timestamp.max(tick(timeframe_map, timestamp_s, f32::NAN, 0.));
        }
        max_candle_timestamp
    }

    /// Builds candles from trade data. When new candle is added, candles are shifted to the left
    /// removing the oldest one.
    /// Returns completed candle timestamp when candle is completed, otherwise returns 0.
    /// If there are multiple candles that are completed the most recent timestamp is returned.
    /// Send 'price' = NaN and 'volume' = 0 to check if shift happened.
    pub fn tick(&mut self, market: &String, timestamp_s: u32, price: f32, volume: f32) -> u32 {
        let timeframe_map = match self.candles.get_mut(market) {
            None => return 0,
            Some(map) => map,
        };
        return tick(timeframe_map, timestamp_s, price, volume);
    }
}

pub fn tick(
    timeframe_map: &mut HashMap<u32, Candles>,
    timestamp_s: u32,
    price: f32,
    volume: f32,
) -> u32 {
    let mut max_candle_timestamp = 0;
    for (timeframe, candles) in timeframe_map.iter_mut() {
        let remainder = timestamp_s % timeframe;
        let candle_timestamp = timestamp_s - remainder + timeframe;
        // Happens when model requires small amount of candles on low timeframes
        if candle_timestamp < candles.timestamp[0] {
            warn!("Network delay {} {} {}", timestamp_s, price, volume);
            // When first trade for new candle is received all candles in hash map are shifted.
            // So if we get a trade that is for previous candle we can skip all of them.
            break;
        }
        let mut id = ((candle_timestamp - candles.timestamp[0]) / timeframe) as usize;
        if id == candles.timestamp.len() {
            max_candle_timestamp = max_candle_timestamp.max(candle_timestamp - timeframe);
            candles.rotate_left();
            id -= 1;
        } else if id > candles.len() {
            debug!("{:#?}", candles);
            candles.debug_candle(candles.len() - 1);
            dbg!(timestamp_s);
            panic!("CandlesBuilder::tick() didn't happen thus it didn't generate a new candle.");
        } else if id < candles.len() - 1 {
            warn!("Network delay {} {} {}", timestamp_s, price, volume);
            // When first trade for new candle is received all candles in hash map are shifted.
            // So if we get a trade that is for previous candle we can skip all of them.
            break;
        }
        if price.is_nan() {
            continue;
        }
        if price > candles.high[id] {
            candles.high[id] = price;
        }
        if price < candles.low[id] {
            candles.low[id] = price;
        }
        candles.close[id] = price;
        candles.volume[id] += volume;
    }
    max_candle_timestamp
}

pub struct CandleAppender {
    timeframe: u32,
    candles: Vec<Candle>,
}

impl CandleAppender {
    pub fn new(timeframe: u32) -> CandleAppender {
        CandleAppender {
            timeframe,
            candles: vec![],
        }
    }

    pub fn tick(&mut self, timestamp_s: u32, price: f32, volume: f32) {
        let timestamp_s = timestamp_s - timestamp_s % self.timeframe + self.timeframe;
        let last_timestamp = match self.candles.last() {
            None => 0,
            Some(candle) => candle.timestamp,
        };
        if timestamp_s > last_timestamp {
            let open = match self.candles.last() {
                None => price,
                Some(candle) => candle.close,
            };
            self.candles.push(Candle {
                timestamp: timestamp_s,
                open,
                high: price.max(open),
                low: price.min(open),
                close: price,
                volume,
            });
        } else {
            let mut last_candle = self.candles.last_mut().unwrap();
            last_candle.close = price;
            last_candle.high.max_mut(price);
            last_candle.low.min_mut(price);
            last_candle.volume += volume;
        }
    }

    pub fn candles(&self) -> &Vec<Candle> {
        &self.candles
    }

    pub fn clear(&mut self) {
        self.candles.clear();
    }
}

#[cfg(test)]
mod t_candle_builder {
    use test_helper::merovingian::candles::Candles;
    use test_helper::merovingian::candles_builder::{CandleAppender, CandlesBuilder};
    use test_helper::*;

    #[test]
    fn t_tick() {
        configure_logging_once();
        let timeframe = CANDLES.timeframe_step();
        CANDLES.check_integrity();
        let mut candles_builder = CandlesBuilder::new();
        let market = "test".to_string();
        let exchange = "test".to_string();
        let len = 16;
        let mut candles = Candles::with_capacity(exchange.clone(), market.clone(), len);
        for i in 0..len {
            candles.push_candle(&CANDLES.get_candle(i).unwrap());
        }
        candles_builder.insert(&market, candles);
        let mut appender = CandleAppender::new(timeframe);
        for i in len..CANDLES.len() {
            let candle = CANDLES.get_candle(i).unwrap();
            appender.tick(candle.timestamp - 45, candle.open, candle.volume / 4.);
            let shifted = candles_builder.tick(
                &market,
                candle.timestamp - 45,
                candle.open,
                candle.volume / 4.,
            ) != 0;
            if i > len {
                assert!(shifted);
            }
            if shifted {
                let mcandles = candles_builder.candles();
                let tcandles = mcandles.get(&market).unwrap();
                let candles = tcandles.get(&timeframe).unwrap();
                for j in 0..len - 1 {
                    let candle = candles.get_candle(j).unwrap();
                    let source = CANDLES.get_candle(i - len + j + 1).unwrap();
                    a_eq!(candle.timestamp, source.timestamp);
                    a_eq!(candle.open, source.open);
                    a_eq!(candle.high, source.high);
                    a_eq!(candle.low, source.low);

                    a_eq!(candle.close, source.close);
                    // Testing like this because of floating point precision error.
                    if source.volume * 1.01 < candle.volume || source.volume * 0.99 > candle.volume
                    {
                        panic!("Volume not in range.")
                    }
                }
            }
            candles_builder.tick(
                &market,
                candle.timestamp - 30,
                candle.high,
                candle.volume / 4.,
            );
            appender.tick(candle.timestamp - 30, candle.high, candle.volume / 4.);
            candles_builder.tick(
                &market,
                candle.timestamp - 15,
                candle.low,
                candle.volume / 4.,
            );
            appender.tick(candle.timestamp - 15, candle.low, candle.volume / 4.);
            candles_builder.tick(
                &market,
                candle.timestamp - 1,
                candle.close,
                candle.volume / 4.,
            );
            appender.tick(candle.timestamp - 1, candle.close, candle.volume / 4.);
        }
        for i in 0..appender.candles().len() {
            let a = &appender.candles()[i];
            let c = CANDLES.get_candle(i + len).unwrap();
            a_eq!(a, &c)
        }
    }
}
