use std::f32::NAN;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, ErrorKind, SeekFrom};
use std::ops::Index;
use std::path::Path;
use std::time::Instant;

use async_compression::tokio::bufread::ZstdDecoder;
use async_compression::tokio::write::ZstdEncoder;
use async_compression::Level;
use mouse::error::{anyhow, Result, ResultCtxExt};
use mouse::ext::PathExt;
use mouse::helpers::{ptr_as_slice, ptr_as_slice_mut};
use mouse::num::NumExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Candles {
    // TODO: Remove timestamp entirely, use it only to create continuous data
    pub timestamp: Vec<u32>,
    pub open: Vec<f32>,
    pub high: Vec<f32>,
    pub low: Vec<f32>,
    pub close: Vec<f32>,
    pub volume: Vec<f32>,
    pub exchange: String,
    pub market: String,
}

#[repr(C)]
#[derive(Serialize, Deserialize, Readable, Writable, PartialEq, Debug, Clone)]
pub struct Candle {
    pub timestamp: u32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub volume: f32,
}

impl Candle {
    /// First candle will be automatically discarded when completing.
    /// Timestamp doesn't need to be divisible by 60.
    pub fn new_partial(timestamp: u32) -> Candle {
        Candle {
            timestamp: timestamp - timestamp % 60,
            open: NAN,
            high: NAN,
            low: NAN,
            close: NAN,
            volume: NAN,
        }
    }
}

impl Index<usize> for Candles {
    type Output = Vec<f32>;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.open,
            1 => &self.high,
            2 => &self.low,
            3 => &self.close,
            4 => &self.volume,
            _ => panic!("wrong index of candle type"),
        }
    }
}

impl Candles {
    /// Creates a candle from specific index.
    pub fn get_candle(&self, i: usize) -> Option<Candle> {
        if i >= self.len() {
            return None;
        }
        Some(Candle {
            timestamp: self.timestamp[i],
            open: self.open[i],
            high: self.high[i],
            low: self.low[i],
            close: self.close[i],
            volume: self.volume[i],
        })
    }

    /// Appends a candle to the file that in format AOS (Array of Structures)
    pub fn append_to_binary_aos(&self, path: &str, candle: &Candle) {
        // writing with format candle,candle instead of timestamps,opens... so that file could
        // easily be appended.
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .unwrap();
        let mut buf_writer = BufWriter::new(file);
        let encoded: Vec<u8> = bincode::serialize(candle).unwrap();
        buf_writer.write(&encoded).unwrap();
    }

    pub fn len(&self) -> usize {
        self.timestamp.len()
    }

    // Writes to binary file in aos format.
    pub fn write_to_binary_aos(&self, path: impl AsRef<Path>) -> Result<()> {
        // writing with format candle,candle instead of timestamps,opens... so that file could
        // easily be appended.
        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        for i in 0..self.timestamp.len() {
            let encoded: Vec<u8> = bincode::serialize(&(
                self.timestamp[i],
                self.open[i],
                self.high[i],
                self.low[i],
                self.close[i],
                self.volume[i],
            ))?;
            buf_writer.write(&encoded)?;
        }
        Ok(())
    }

    /// Writes to a file with best possible layout for fast reads.
    pub async fn write_fast(&self, path: impl AsRef<Path>) -> Result<()> {
        trace!("Compressing candles...");
        let mut writer = ZstdEncoder::with_quality(
            tokio::io::BufWriter::new(tokio::fs::File::create(&path).await?),
            Level::Precise(22),
        );
        unsafe {
            writer.write_u64_le(self.len() as u64).await?;
            let data = ptr_as_slice(self.timestamp.as_ptr(), 4 * self.timestamp.len());
            writer.write_all(data).await?;
            let data = ptr_as_slice(self.open.as_ptr(), 4 * self.open.len());
            writer.write_all(data).await?;
            let data = ptr_as_slice(self.high.as_ptr(), 4 * self.high.len());
            writer.write_all(data).await?;
            let data = ptr_as_slice(self.low.as_ptr(), 4 * self.low.len());
            writer.write_all(data).await?;
            let data = ptr_as_slice(self.close.as_ptr(), 4 * self.close.len());
            writer.write_all(data).await?;
            let data = ptr_as_slice(self.volume.as_ptr(), 4 * self.volume.len());
            writer.write_all(data).await?;
        }
        writer.shutdown().await?;

        Ok(())
    }

    pub fn push_candle(&mut self, candle: &Candle) {
        self.push(
            candle.timestamp,
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume,
        );
    }

    pub fn set_candle_partial(&mut self, id: usize) {
        self.timestamp[id] = self.timestamp[id - 1] + self.timeframe_step();
        self.open[id] = self.close[id - 1];
        self.high[id] = self.close[id - 1];
        self.low[id] = self.close[id - 1];
        self.close[id] = self.close[id - 1];
        self.volume[id] = 0.;
    }
    // Shifts candles by one and initialises last candle to partial.
    pub fn rotate_left(&mut self) {
        self.timestamp.rotate_left(1);
        self.open.rotate_left(1);
        self.high.rotate_left(1);
        self.low.rotate_left(1);
        self.close.rotate_left(1);
        self.volume.rotate_left(1);
        *self.timestamp.last_mut().unwrap() =
            self.timestamp[self.len() - 2] + self.timeframe_step();
        *self.open.last_mut().unwrap() = self.close[self.len() - 2];
        *self.high.last_mut().unwrap() = self.close[self.len() - 2];
        *self.low.last_mut().unwrap() = self.close[self.len() - 2];
        *self.close.last_mut().unwrap() = self.close[self.len() - 2];
        *self.volume.last_mut().unwrap() = 0.;
    }

    pub fn push(
        &mut self,
        timestamp: u32,
        open: f32,
        high: f32,
        low: f32,
        close: f32,
        volume: f32,
    ) {
        self.timestamp.push(timestamp);
        self.open.push(open);
        self.high.push(high);
        self.low.push(low);
        self.close.push(close);
        self.volume.push(volume);
    }

    pub fn fix_discontinuous_data(&mut self, start_id: usize, timeframe_hint: u32) {
        for i in start_id + 1..self.timestamp.len() {
            let dif = self.timestamp[i] - self.timestamp[i - 1];
            if dif != timeframe_hint {
                self.timestamp
                    .insert(i, self.timestamp[i - 1] + timeframe_hint);
                self.open.insert(i, self.close[i - 1]);
                self.high.insert(i, self.close[i - 1]);
                self.low.insert(i, self.close[i - 1]);
                self.close.insert(i, self.close[i - 1]);
                self.volume.insert(i, 0.);
                self.fix_discontinuous_data(i, timeframe_hint);
                return;
            }
        }
    }

    pub fn timeframe_step(&self) -> u32 {
        self.timestamp[1] - self.timestamp[0]
    }

    pub fn truncate(&mut self, len: usize) {
        self.open.truncate(len);
        self.high.truncate(len);
        self.low.truncate(len);
        self.close.truncate(len);
        self.volume.truncate(len);
        self.timestamp.truncate(len);
    }

    /// Increases timeframe, lets say from 1 min to 5 min reducing number of candles. Returns count
    /// number of generated candles or None if same timeframe already. Doesn't include partial
    /// candle if there aren't enough candles at the end of vector.
    pub fn increase_timeframe(
        &self,
        destination: &mut Candles,
        timeframe_step: u32,
        partial: bool,
    ) -> usize {
        let original_step = self.timestamp[1] - self.timestamp[0];
        if timeframe_step % original_step != 0 || timeframe_step == original_step {
            error!(
                "Invalid timeframe step in increase timeframe, original step {}, requested step {}",
                original_step, timeframe_step,
            );
            panic!(
                "Invalid timeframe step in increase timeframe, original step {}, requested step {}",
                original_step, timeframe_step,
            );
        }
        // Starting at time that is divisible by step so that monthly timeframe starts at beginning
        // of a month.
        let mut offset = std::usize::MAX;
        for t in &self.timestamp {
            if t % timeframe_step == original_step {
                offset = ((t - self.timestamp[0]) / original_step) as usize;
                break;
            }
        }
        let compacted_len =
            (*self.timestamp.last().unwrap() - self.timestamp[0] - offset as u32) / timeframe_step;

        if compacted_len > destination.len() as u32 {
            error!(
                "Not enough space provided in increase_timeframe, {} required, {} provided",
                compacted_len,
                destination.len()
            );
            panic!(
                "Not enough space provided in increase_timeframe, {} required, {} provided",
                compacted_len,
                destination.len()
            );
        }
        let mut high = self.high[offset];
        let mut low = self.low[offset];
        let mut volume = self.volume[offset];
        let timeframe_difference = (timeframe_step / original_step) as usize;
        let mut id = 0;
        for i in offset + 1..self.timestamp.len() {
            if high < self.high[i] {
                high = self.high[i];
            }
            if low > self.low[i] {
                low = self.low[i];
            }
            volume += self.volume[i];

            if (i + 1) % timeframe_difference == offset {
                id = (i - offset) / timeframe_difference;
                // Wow, it throws overflow exception if we change order.
                destination.open[id] = self.open[i + 1 - timeframe_difference];
                destination.high[id] = high;
                destination.low[id] = low;
                destination.close[id] = self.close[i];
                destination.volume[id] = volume;
                destination.timestamp[id] = self.timestamp[i];
                volume = 0.;
                low = std::f32::MAX;
                high = std::f32::MIN;
            }
        }
        // Candles are partial
        if partial && low != f32::MAX {
            id += 1;
            destination.open[id] = destination.close[id - 1];
            destination.high[id] = high;
            destination.low[id] = low;
            destination.close[id] = *self.close.last().unwrap();
            destination.volume[id] = volume;
            destination.timestamp[id] = destination.timestamp[id - 1] + timeframe_step;
        }
        id + 1
    }

    /// Checks if candles are valid:
    /// Candle must start with open price equal to the close price of prevous candle.
    /// Low must be the lowest price for candle.
    /// High must be the highest price for candle.
    /// Candles must be continuous i.e. the difference between two timestamp of neighbour candles
    /// must always be the same and equal to timeframe.
    pub fn check_integrity(&self) {
        assert!(
            self.low[0] <= self.open[0]
                && self.low[0] <= self.high[0]
                && self.low[0] <= self.close[0]
        );
        assert!(
            self.high[0] >= self.open[0]
                && self.high[0] >= self.low[0]
                && self.high[0] >= self.close[0]
        );
        let timeframe = self.timeframe_step();
        for i in 1..self.len() {
            assert_eq!(self.timestamp[i] - self.timestamp[i - 1], timeframe);
            assert_eq!(self.open[i], self.close[i - 1]);
            assert!(
                self.low[i] <= self.open[i]
                    && self.low[i] <= self.high[i]
                    && self.low[i] <= self.close[i]
            );
            assert!(
                self.high[i] >= self.open[i]
                    && self.high[i] >= self.low[i]
                    && self.high[i] >= self.close[i]
            );
        }
    }

    /// Applies all possible fixes for candles but they may still be bad if the source of candles is
    /// bad. Candles should always be fixed because exchanges don't provide the same format for
    /// candles (they even brake their own rules).
    /// See check-integrity documentation for more info about integrity.
    pub fn fix_integrity(&mut self, timeframe_hint: u32) {
        trace!("Fixing candles integrity...");
        let mut fixer = CandleFixer::new(timeframe_hint, self.timestamp[0], self.close[0]);
        let mut i = 0;
        loop {
            if i >= self.len() {
                break;
            }

            match fixer.fix(self.get_candle(i).unwrap()) {
                Ok(candle) => {
                    if let Some(candle) = candle {
                        self.timestamp[i] = candle.timestamp;
                        self.open[i] = candle.open;
                        self.high[i] = candle.high;
                        self.low[i] = candle.low;
                        self.close[i] = candle.close;
                        self.volume[i] = candle.volume;
                        i += 1;
                    } else {
                        self.timestamp.remove(i);
                        self.open.remove(i);
                        self.high.remove(i);
                        self.low.remove(i);
                        self.close.remove(i);
                        self.volume.remove(i);
                    }
                }
                Err(candles) => {
                    let timestamp = self.timestamp.split_off(i + 1);
                    let open = self.open.split_off(i + 1);
                    let high = self.high.split_off(i + 1);
                    let low = self.low.split_off(i + 1);
                    let close = self.close.split_off(i + 1);
                    let volume = self.volume.split_off(i + 1);
                    self.truncate(self.len() - 1);
                    for candle in &candles {
                        self.push_candle(candle);
                    }
                    self.timestamp.extend_from_slice(&timestamp);
                    self.open.extend_from_slice(&open);
                    self.high.extend_from_slice(&high);
                    self.low.extend_from_slice(&low);
                    self.close.extend_from_slice(&close);
                    self.volume.extend_from_slice(&volume);
                    i += candles.len();
                }
            }
        }
    }

    pub fn trim(&mut self, start_timestamp_s: u32, end_timestamp_s: u32) {
        warn!("trimming candles");
        let timeframe = self.timeframe_step();
        let start_timestamp_s = start_timestamp_s - start_timestamp_s % timeframe;
        let end_timestamp_s = end_timestamp_s - end_timestamp_s % timeframe;
        let excess_count = (self.timestamp.last().unwrap() - end_timestamp_s) / timeframe;
        self.truncate(self.len() - excess_count as usize);
        let offset = ((start_timestamp_s - self.timestamp[0]) / timeframe) as usize;
        self.timestamp.copy_within(offset.., 0);
        self.open.copy_within(offset.., 0);
        self.high.copy_within(offset.., 0);
        self.low.copy_within(offset.., 0);
        self.close.copy_within(offset.., 0);
        self.volume.copy_within(offset.., 0);
        self.truncate(self.len() - offset);
    }
}

impl Candles {
    pub fn new() -> Candles {
        Candles {
            timestamp: Vec::new(),
            open: Vec::new(),
            high: Vec::new(),
            low: Vec::new(),
            close: Vec::new(),
            volume: Vec::new(),
            exchange: String::new(),
            market: String::new(),
        }
    }

    pub fn new_partial(
        exchange: String,
        market: String,
        timestamp: u32,
        price: f32,
        volume: f32,
    ) -> Candles {
        Candles {
            timestamp: vec![timestamp],
            open: vec![std::f32::NAN],
            high: vec![price],
            low: vec![price],
            close: vec![price],
            volume: vec![volume],
            exchange,
            market,
        }
    }

    pub fn with_capacity(exchange: String, market: String, count: usize) -> Candles {
        Candles {
            timestamp: Vec::with_capacity(count),
            open: Vec::with_capacity(count),
            high: Vec::with_capacity(count),
            low: Vec::with_capacity(count),
            close: Vec::with_capacity(count),
            volume: Vec::with_capacity(count),
            exchange,
            market,
        }
    }

    /// Creates new candles with specified length. All of the fields will be filled with default
    /// value (for f32 it is 0.0) up until count.
    pub fn with_default_value(exchange: String, market: String, count: usize) -> Candles {
        Candles {
            timestamp: vec![Default::default(); count],
            open: vec![Default::default(); count],
            high: vec![Default::default(); count],
            low: vec![Default::default(); count],
            close: vec![Default::default(); count],
            volume: vec![Default::default(); count],
            exchange,
            market,
        }
    }

    pub async fn read(path: impl AsRef<Path>) -> Result<Candles> {
        let now = Instant::now();
        let path = path.as_ref();
        let mut path_mut = path.to_owned();
        for i in 0..3 {
            let extension = match path.extension() {
                None => match i {
                    0 => {
                        path_mut.set_extension("zstd");
                        "zstd"
                    }
                    1 => {
                        path_mut.set_extension("bin");
                        "bin"
                    }
                    2 => {
                        path_mut.set_extension("csv");
                        "csv"
                    }
                    _ => panic!("Unknown file extension `{}`", path.display()),
                },
                Some(extension) => extension.to_str().unwrap(),
            };
            if !path_mut.exists_async().await {
                continue;
            }
            let candles = match extension {
                "zstd" => Candles::read_fast(&path_mut).await?,
                "bin" => Candles::from_binary_aos(&path_mut)?,
                "csv" => Candles::read_from_csv(&path_mut)?,
                _ => continue,
            };
            println!("read in: {} ms", now.elapsed().as_millis());
            return Ok(candles);
        }
        Err(std::io::Error::new(
            ErrorKind::NotFound,
            anyhow!("File not found `{}`", path.display()),
        )
        .into())
    }

    pub async fn read_fast(path: impl AsRef<Path>) -> Result<Candles> {
        let (exchange, market) = path_to_exchange_and_market(&path);
        let mut reader = ZstdDecoder::new(tokio::io::BufReader::new(
            tokio::fs::File::open(&path).await?,
        ));
        let len = reader.read_u64_le().await? as usize;
        unsafe {
            let mut timestamp = Vec::with_capacity(len);
            timestamp.set_len(len);
            let data = ptr_as_slice_mut(timestamp.as_mut_ptr(), 4 * timestamp.len());
            reader.read_exact(data).await?;

            let mut open = Vec::with_capacity(len);
            open.set_len(len);
            let data = ptr_as_slice_mut(open.as_mut_ptr(), 4 * open.len());
            reader.read_exact(data).await?;

            let mut high = Vec::with_capacity(len);
            high.set_len(len);
            let data = ptr_as_slice_mut(high.as_mut_ptr(), 4 * high.len());
            reader.read_exact(data).await?;

            let mut low = Vec::with_capacity(len);
            low.set_len(len);
            let data = ptr_as_slice_mut(low.as_mut_ptr(), 4 * low.len());
            reader.read_exact(data).await?;

            let mut close = Vec::with_capacity(len);
            close.set_len(len);
            let data = ptr_as_slice_mut(close.as_mut_ptr(), 4 * close.len());
            reader.read_exact(data).await?;

            let mut volume = Vec::with_capacity(len);
            volume.set_len(len);
            let data = ptr_as_slice_mut(volume.as_mut_ptr(), 4 * volume.len());
            reader.read_exact(data).await?;

            Ok(Candles {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                exchange,
                market,
            })
        }
    }

    pub fn read_from_csv(path: impl AsRef<Path>) -> Result<Candles> {
        let file = File::open(&path)?;
        let file = BufReader::new(file);
        let mut line_iter = file.lines().filter_map(|result| result.ok());
        // wtf man
        let count = line_iter
            .next()
            .unwrap()
            .split(";")
            .next()
            .unwrap()
            .parse()?;
        let (exchange, market) = path_to_exchange_and_market(&path);
        let mut candles = Candles::with_capacity(exchange, market, count);

        for line in line_iter {
            let mut elements = line.split(";");
            candles
                .timestamp
                .push((elements.next().unwrap().parse::<u64>()? / 1000) as u32);
            candles.open.push(elements.next().unwrap().parse()?);
            candles.high.push(elements.next().unwrap().parse()?);
            candles.low.push(elements.next().unwrap().parse()?);
            candles.close.push(elements.next().unwrap().parse()?);
            candles.volume.push(elements.next().unwrap().parse()?);
        }
        Ok(candles)
    }

    pub fn from_binary_soa(path: impl AsRef<Path>) -> Result<Candles> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::<u8>::new();
        file.read_to_end(&mut buffer)?;
        let candles = bincode::deserialize(&buffer)?;
        Ok(candles)
    }

    pub fn from_binary_aos(path: impl AsRef<Path>) -> Result<Candles> {
        let path = path.as_ref();
        let mut file = File::open(path).context(path.to_str().unwrap().to_string())?;

        let mut buffer = Vec::<u8>::with_capacity(file.seek(SeekFrom::End(0))? as usize);
        file.seek(SeekFrom::Start(0))?;
        file.read_to_end(&mut buffer)?;
        let (exchange, market) = path_to_exchange_and_market(path);
        let mut candles = Candles::with_capacity(exchange, market, buffer.len() / 24);
        let (_head, body, _tail) = unsafe { buffer.align_to::<Candle>() };
        for i in 0..buffer.len() / 24 {
            // let timestamp: u32 = unsafe { std::mem::transmute_copy(&buffer[i * 24..i * 24 + 4])
            // }; println!("{}", body[i].open);
            // let candle: Candle = bincode::deserialize(&buffer[i * 24..i * 24 + 24]).unwrap();
            let candle = &body[i];
            candles.timestamp.push(candle.timestamp);
            candles.open.push(candle.open);
            candles.high.push(candle.high);
            candles.low.push(candle.low);
            candles.close.push(candle.close);
            candles.volume.push(candle.volume);
        }
        Ok(candles)
    }

    pub fn debug_candle(&self, i: usize) {
        println!(
            "{} {} {} {} {}",
            self.timestamp[i], self.open[i], self.high[i], self.low[i], self.close[i],
        );
    }
}

pub struct CandleFixer {
    timeframe: u32,
    last_timestamp: u32,
    last_close: f32,
}

impl CandleFixer {
    pub fn new(timeframe: u32, first_timestamp: u32, first_close: f32) -> CandleFixer {
        CandleFixer {
            timeframe,
            last_timestamp: first_timestamp - timeframe,
            last_close: first_close,
        }
    }

    pub fn fix(&mut self, mut candle: Candle) -> Result<Option<Candle>, Vec<Candle>> {
        let gap = (candle.timestamp as i64 - self.last_timestamp as i64) / self.timeframe as i64;
        if gap == 1 {
            self.fix_candle(&mut candle);
            self.last_timestamp = candle.timestamp;
            self.last_close = candle.close;
            Ok(Some(candle))
        } else if gap > 1 {
            let mut candles = Vec::with_capacity(gap as usize);
            self.fix_candle(&mut candle);
            self.last_timestamp = self.last_timestamp + self.timeframe;
            self.last_close = candle.close;
            candle.timestamp = self.last_timestamp;
            candles.push(candle.clone());
            for _ in 1..gap {
                self.last_timestamp = self.last_timestamp + self.timeframe;
                candle.timestamp = self.last_timestamp;
                candle.open = self.last_close;
                candles.push(candle.clone());
            }
            Err(candles)
        } else if gap == 0 {
            Ok(None)
        } else {
            warn!("Candle {:?} already processed {} candles ago", candle, -gap);
            Ok(None)
        }
    }

    fn fix_candle(&mut self, candle: &mut Candle) {
        candle.open = self.last_close;
        candle.low.min_mut(candle.open);
        candle.low.min_mut(candle.high);
        candle.low.min_mut(candle.close);
        candle.high.max_mut(candle.open);
        candle.high.max_mut(candle.low);
        candle.high.max_mut(candle.close);
    }
}

fn path_to_exchange_and_market(path: impl AsRef<Path>) -> (String, String) {
    let path = Path::new(path.as_ref()).canonicalize().ok().unwrap();

    let file_name = path.file_name().unwrap().to_str().unwrap();
    let end_pos = file_name.rfind('.').unwrap();
    let market = String::from(&file_name[0..end_pos]);
    let exchange_dir = path.parent().unwrap().parent().unwrap();
    let exchange = String::from(exchange_dir.file_name().unwrap().to_str().unwrap());
    (exchange, market)
}

#[cfg(test)]
mod t_candles {
    use mouse::error::Result;
    use test_helper::*;

    use crate::candles::{CandleFixer, Candles};

    #[test]
    fn t_candle_fixer() -> Result<()> {
        configure_logging_once();
        let candles = Candles::from_binary_aos("../test_data/XBTUSD1m-short.bin")?;
        let mut candle_fixer = CandleFixer::new(60, candles.timestamp[0], candles.close[0]);
        let mut fixed = Candles::new();
        for i in 0..candles.len() {
            let candle = candles.get_candle(i).unwrap();
            match candle_fixer.fix(candle) {
                Ok(candle) => fixed.push_candle(&candle.unwrap()),
                Err(candles) => {
                    for candle in candles {
                        fixed.push_candle(&candle);
                    }
                }
            }
        }
        fixed.check_integrity();
        Ok(())
    }

    #[test]
    fn t_increase_timeframe() -> Result<()> {
        // logs don't work in tests right now but there is an active issue about it
        configure_logging_once();
        let candles = Candles::from_binary_aos("../test_data/XBTUSD1m-short.bin")?;
        assert_eq!(candles.len(), 1000);
        assert_eq!(candles.close[0], 7248.);
        assert_eq!(*candles.close.last().unwrap(), 7364.5);
        assert_eq!(candles.timestamp[0], 1586205300);
        assert_eq!(*candles.timestamp.last().unwrap(), 1586265240);
        // for i in 0..CANDLES.len() {
        //     debug!(
        //         "{:>6} {:>6} {:>6} {:>6} {:>6} {:>6} {:>10}",
        //         i + 1,
        //         CANDLES.timestamp[i],
        //         CANDLES.open[i],
        //         CANDLES.high[i],
        //         CANDLES.low[i],
        //         CANDLES.close[i],
        //         CANDLES.volume[i],
        //     )
        // }
        let mut reduced = Candles::with_default_value("".into(), "".into(), candles.len());
        let len = candles.increase_timeframe(&mut reduced, 120, true);
        trace!("2 min timeframe");
        a_eq!(len, 500);
        a_eq!(reduced.timestamp[0], candles.timestamp[1]);
        a_eq!(reduced.open[0], candles.open[0]);
        a_eq!(reduced.high[0], 7249.5);
        a_eq!(reduced.low[0], 7247.);
        a_eq!(reduced.close[0], candles.close[1]);
        a_eq!(reduced.volume[0], candles.volume[0] + candles.volume[1]);

        a_eq!(reduced.timestamp[499], candles.timestamp[999]);
        a_eq!(reduced.open[499], candles.open[998]);
        a_eq!(reduced.high[499], 7369.);
        a_eq!(reduced.low[499], 7359.5);
        a_eq!(reduced.close[499], candles.close[999]);
        a_eq!(
            reduced.volume[499],
            candles.volume[999] + candles.volume[998]
        );

        let id = candles.increase_timeframe(&mut reduced, 420, true);
        a_eq!(reduced.timestamp[0] % reduced.timeframe_step(), 0);
        a_eq!(id, 142);
        trace!("7 min timeframe");
        // remember that candles have offset if they don't align correctly
        a_eq!(reduced.timestamp[0], candles.timestamp[12]);
        a_eq!(reduced.open[0], candles.open[6]);
        a_eq!(reduced.high[0], 7256.5);
        a_eq!(reduced.low[0], 7239.5);
        a_eq!(reduced.close[0], candles.close[12]);
        a_eq!(reduced.volume[0], 6108836.);

        a_eq!(reduced.timestamp[5], candles.timestamp[47]);
        a_eq!(reduced.open[5], candles.open[41]);
        a_eq!(reduced.high[5], 7255.);
        a_eq!(reduced.low[5], 7241.);
        a_eq!(reduced.close[5], candles.close[47]);
        a_eq!(reduced.volume[5], 3375923.);

        let id = candles.increase_timeframe(&mut reduced, 123 * 60, true);
        a_eq!(id, 9);
        a_eq!(reduced.timestamp[0], candles.timestamp[5 + 123 - 1]);
        a_eq!(reduced.open[0], candles.open[5]);
        let mut high = 0.;
        let mut low = std::f32::MAX;
        let mut volume = 0.;
        for i in 5..5 + 123 {
            volume += candles.volume[i];
            if high < candles.high[i] {
                high = candles.high[i]
            }
            if low > candles.low[i] {
                low = candles.low[i];
            }
        }
        a_eq!(reduced.high[0], high);
        a_eq!(reduced.low[0], low);
        a_eq!(reduced.close[0], candles.close[5 + 123 - 1]);
        a_eq!(reduced.volume[0], volume);

        a_eq!(reduced.timestamp[7], candles.timestamp[8 * 123 + 5 - 1]);
        a_eq!(reduced.open[7], candles.open[7 * 123 + 5]);
        let mut high = 0.;
        let mut low = std::f32::MAX;
        let mut volume = 0.;
        for i in 7 * 123 + 5..8 * 123 + 5 {
            volume += candles.volume[i];
            if high < candles.high[i] {
                high = candles.high[i]
            }
            if low > candles.low[i] {
                low = candles.low[i];
            }
        }
        a_eq!(reduced.high[7], high);
        a_eq!(reduced.low[7], low);
        a_eq!(reduced.close[7], candles.close[8 * 123 + 5 - 1]);
        a_eq!(reduced.volume[7], volume);

        a_eq!(
            reduced.timestamp[8],
            reduced.timestamp[7] + reduced.timeframe_step()
        );
        a_eq!(reduced.open[8], candles.open[8 * 123 + 5]);
        let mut high = 0.;
        let mut low = std::f32::MAX;
        let mut volume = 0.;
        for i in 8 * 123 + 5..candles.len() {
            volume += candles.volume[i];
            if high < candles.high[i] {
                high = candles.high[i]
            }
            if low > candles.low[i] {
                low = candles.low[i];
            }
        }
        a_eq!(reduced.high[8], high);
        a_eq!(reduced.low[8], low);
        a_eq!(reduced.close[8], *candles.close.last().unwrap());
        a_eq!(reduced.volume[8], volume);
        Ok(())
    }
}
