#![deny(unused_must_use)]
// #![feature(num_as_ne_bytes)]
#![feature(thread_id_value)]

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Clap;
use config::{get_exchange_config, select_exchange, CONFIG};
use half::f16;
use iaas::mouse::num::traits::Float;
use iaas::mysql::{get_model_source_id, insert_model_source, insert_model_values};
use memmap2::MmapOptions;
use merovingian::candles::Candles;
use merovingian::candles_builder::CandleAppender;
use merovingian::minable_models::Trade;
use merovingian::speedy::{IsEof, LittleEndian, Readable, Writable};
use mouse::error::Result;
use mouse::log::*;
use mouse::macros::futures_util::io::ErrorKind;
use mouse::num::traits::{ToPrimitive, Zero};
use mouse::num::NumExt;
use mouse::time::{IntoDateTime, Timestamp};
use nebuchadnezzar::core::client::SuperClient;
use nebuchadnezzar::core::futures_util::StreamExt;
use nebuchadnezzar::core::paginators::BasicPaginator;
use nebuchadnezzar::core::requests::CandlesGetRequest;
use nebuchadnezzar::core::Credentials;
use residual_self_image::seek_report::find_max;
use tokio::fs;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, BufReader, BufWriter};
use tokio::task::spawn_blocking;

#[derive(Clap)]
#[clap(version, about, author)]
pub struct Args {
    #[clap(long, short, parse(from_os_str), default_value = "config.yaml")]
    /// Path to config file.
    config: PathBuf,
    #[clap(subcommand)]
    sub_command: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Fetch(FetchArgs),
    /// Insert things into Database that are hard to insert.
    Insert(InsertCommand),
}

#[derive(Clap)]
struct InsertArgs {
    #[clap(subcommand)]
    _command: InsertCommand,
}

#[derive(Clap)]
enum InsertCommand {
    SourceCode(SourceCodeArgs),
    VariableValues(VariableValuesArgs),
}

#[derive(Clap)]
struct SourceCodeArgs {
    #[clap(long, short)]
    /// Name of a model to insert
    name: String,
}

#[derive(Clap)]
struct VariableValuesArgs {
    #[clap(long, short)]
    /// Model name
    name: String,
    #[clap(long, short)]
    /// Values to insert
    values: Vec<f32>,
}

#[derive(Clap)]
struct FetchConfig {
    #[clap(long, short)]
    exchange: String,
    #[clap(long, short)]
    symbol: String,
    #[clap(long, short, parse(try_from_str = parse_date))]
    /// Start from this date and time with "%d.%m.%Y. %H:%M:%S%.f" format.
    from: DateTime<Utc>,
    #[clap(long, short, parse(try_from_str = parse_date))]
    /// End date and time (excluding) with "%d.%m.%Y. %H:%M:%S%.f" format or "now".
    to: DateTime<Utc>,
}

#[derive(Clap)]
struct FetchArgs {
    #[clap(subcommand)]
    fetch_command: FetchCommand,
    #[clap(flatten)]
    args: FetchConfig,
}

#[derive(Clap)]
enum FetchCommand {
    Candles(CandlesArgs),
}

#[derive(Clap)]
struct CandlesArgs {
    #[clap(long, short)]
    // Timeframe in seconds, 60 means 1 minute.
    timeframe: u32,
}

pub const DATE_FORMAT: &'static str = "%d.%m.%Y. %H:%M:%S%.f";
pub fn parse_date(s: &str) -> std::result::Result<DateTime<Utc>, &'static str> {
    if s == "now" {
        return Ok(Utc::now());
    }
    return match NaiveDateTime::parse_from_str(s, DATE_FORMAT) {
        Ok(dt) => Ok(DateTime::from_utc(dt, Utc)),
        Err(e) => {
            println!(
                "{:?}, input format should be in this '{}' format.",
                e, DATE_FORMAT
            );
            Err("")
        }
    };
}

struct Bounds<T> {
    start: T,
    end: T,
}

async fn load_bounds<'a, T: Readable<'a, LittleEndian> + Clone>(
    path: &PathBuf,
) -> Result<Option<Bounds<T>>> {
    if fs::metadata(&path).await.is_err() {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)?;
    let start = T::read_from_stream_unbuffered(&mut file)?;
    let mut end = start.clone();
    loop {
        match T::read_from_stream_unbuffered(&mut file) {
            Ok(data) => end = data,
            Err(e) => {
                if e.is_eof() {
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    Ok(Some(Bounds { start, end }))
}

async fn load_candles(
    tmp_path: impl AsRef<Path>,
    client: &Box<dyn SuperClient>,
    symbol: String,
    start: u32,
    end: u32,
    fetching_timeframe: u32,
    target_timeframe: u32,
) -> Result<()> {
    unimplemented!()
    // let mut stream;
    // if target_timeframe == u32::MAX {
    //     panic!("Unsupported timeframe")
    // } else {
    //     stream = client
    //         .paginate_candles(BasicPaginator::new_super(
    //             start,
    //             end,
    //             fetching_timeframe,
    //             move |state| {
    //                 // trace!("{}% completed", (state.end - start) as f32 / (state.i - start) as
    // f32);                 Ok(CandlesGetRequest {
    //                     timeframe: fetching_timeframe,
    //                     symbol: symbol.clone(),
    //                     count: Some(state.count),
    //                     start_time: Some(state.i.into_date_time()),
    //                     end_time: None,
    //                 })
    //             },
    //         ))
    //         .map(|result| {
    //             result.map(|response| {
    //                 response
    //                     .into_iter()
    //                     .map(|candle| {
    //                         let timestamp_ns = candle.timestamp.timestamp_ns() - 1;
    //                         vec![
    //                             Trade {
    //                                 timestamp_ns,
    //                                 price: candle.open.to_f32().unwrap(),
    //                                 amount: 0.0,
    //                             },
    //                             Trade {
    //                                 timestamp_ns,
    //                                 price: candle.high.to_f32().unwrap(),
    //                                 amount: 0.0,
    //                             },
    //                             Trade {
    //                                 timestamp_ns,
    //                                 price: candle.low.to_f32().unwrap(),
    //                                 amount: 0.0,
    //                             },
    //                             Trade {
    //                                 timestamp_ns,
    //                                 price: candle.close.to_f32().unwrap(),
    //                                 amount: candle.volume.to_f32().unwrap(),
    //                             },
    //                         ]
    //                         .into_iter()
    //                     })
    //                     .flatten()
    //             })
    //         })
    //         .boxed_local();
    // }
    // let mut writer = std::io::BufWriter::new(
    //     std::fs::OpenOptions::new()
    //         .create(true)
    //         .write(true)
    //         .append(true)
    //         .open(tmp_path)?,
    // );
    // let mut candle_appender = CandleAppender::new(target_timeframe);
    // while let Some(trades) = stream.next().await {
    //     let trades = trades?;
    //     for trade in trades {
    //         candle_appender.tick(
    //             ((trade.timestamp_ns - 1) / 1_000_000_000) as u32,
    //             trade.price,
    //             trade.amount,
    //         );
    //     }
    //     for candle in candle_appender.candles() {
    //         Writable::write_to_stream(candle, &mut writer)?;
    //     }
    //     trace!(
    //         "{}",
    //         candle_appender.candles()[0].timestamp.into_date_time()
    //     );
    //     candle_appender.clear();
    // }
    // Ok(())
}

async fn merge_files(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let mut reader = BufReader::new(File::open(&src).await?);
    let mut writer = BufWriter::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(dest)
            .await?,
    );
    tokio::io::copy(&mut reader, &mut writer).await?;
    tokio::fs::remove_file(&src).await?;
    Ok(())
}

async fn fetch_candles(fetch_args: FetchConfig, candles_args: CandlesArgs) -> Result<()> {
    let exchange = nebuchadnezzar::exchanges()
        .into_iter()
        .find(|x| x.name() == fetch_args.exchange)
        .expect("Valid exchange name");
    if candles_args.timeframe == 0 {
        panic!("Invalid timeframe");
    }
    let mut timeframe = u32::MAX;
    for t in exchange.client_capability().timeframes.iter().rev() {
        if candles_args.timeframe % t == 0 {
            timeframe = *t;
            break;
        }
    }
    let mut client = exchange.new_client_dyn();
    if let Some(config) = get_exchange_config() {
        if !config.api_key.is_empty() {
            client.authenticate(Credentials::new(&config.api_key, &config.api_secret))?;
        }
    }
    let mut relative_path = PathBuf::from("candles");
    relative_path.push(&fetch_args.symbol);
    let final_path = config::data_file_path(&fetch_args.exchange, relative_path, "bin");
    let mut tmp_path = final_path.clone();
    tmp_path.set_extension("tmp");
    match Candles::read(&final_path).await {
        Ok(candles) => {
            candles.write_to_binary_aos(&final_path)?;
            let bounds = Bounds {
                start: candles.timestamp[0],
                end: *candles.timestamp.last().unwrap(),
            };
            drop(candles);
            let mut start = fetch_args.from.timestamp_s();
            let mut end = bounds.start;
            if start < end {
                load_candles(
                    &tmp_path,
                    &client,
                    fetch_args.symbol.clone(),
                    start,
                    end,
                    timeframe,
                    candles_args.timeframe,
                )
                .await?;
                merge_files(&final_path, &tmp_path).await?;
            }
            start = bounds.end;
            end = fetch_args.to.timestamp_s();
            load_candles(
                &tmp_path,
                &client,
                fetch_args.symbol.clone(),
                start,
                end,
                timeframe,
                candles_args.timeframe,
            )
            .await?;
            merge_files(&tmp_path, &final_path).await?;
        }
        Err(e) => match e.downcast::<std::io::Error>() {
            Ok(e) => match e.kind() {
                ErrorKind::NotFound => {
                    load_candles(
                        &tmp_path,
                        &client,
                        fetch_args.symbol,
                        fetch_args.from.timestamp_s(),
                        fetch_args.to.timestamp_s(),
                        timeframe,
                        candles_args.timeframe,
                    )
                    .await?;
                    tokio::fs::rename(tmp_path, &final_path).await?;
                }
                _ => return Err(e.into()),
            },
            Err(e) => return Err(e),
        },
    }
    optimize_candles(timeframe, &final_path).await?;
    Ok(())
}

async fn optimize_candles(timeframe: u32, final_path: impl AsRef<Path>) -> Result<()> {
    trace!("Optimizing candles...");
    let mut candles = Candles::from_binary_aos(&final_path)?;
    candles.fix_integrity(timeframe);
    let mut path = final_path.as_ref().to_owned();
    path.set_extension("zstd");
    candles.write_fast(&path).await?;
    trace!("Removing old file...");
    tokio::fs::remove_file(&final_path).await?;

    Ok(())
}

fn a() -> Result<()> {
    return Err(std::io::Error::from_raw_os_error(22).into());
}

// 3768677183
// 41033 ms
fn map_populate(path: &str) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mmap = unsafe { MmapOptions::new().populate().map(&file)? };
    let mut sum: u32 = 0;
    for i in mmap.as_ref() {
        sum = sum.wrapping_add(*i as u32);
    }
    println!("{}", sum);
    Ok(())
}

// 3768677183
// 28905 ms
fn map_populate_mt(path: &str) -> Result<()> {
    use rayon::prelude::*;
    let mut file = std::fs::File::open(path)?;
    let mmap = unsafe { MmapOptions::new().populate().map(&file)? };
    let sum: u32 = mmap
        .as_ref()
        .into_par_iter()
        .fold(|| 0u32, |acc, x| acc.wrapping_add(*x as u32))
        .sum();
    println!("{}", sum);
    Ok(())
}
// 19063 ms SSD
// 40308 ms HDD
fn map(path: &str) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let mut sum: u32 = 0;
    for i in mmap.as_ref() {
        sum = sum.wrapping_add(*i as u32);
    }
    println!("{}", sum);
    Ok(())
}

// 19402 ms SSD
// 165026 ms HDD
fn map_rayon(path: &str) -> Result<()> {
    use rayon::prelude::*;
    let file = std::fs::File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let sum: u32 = mmap
        .as_ref()
        .into_par_iter()
        .fold(|| 0u32, |acc, x| acc.wrapping_add(*x as u32))
        .sum();
    println!("{}", sum);
    Ok(())
}

// 3768677183
// 18979 ms
fn read128(path: &str) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut sum: u32 = 0;
    let mut block = vec![0u8; 128];
    while file.read_exact(&mut block).is_ok() {
        for i in &block {
            sum = sum.wrapping_add(*i as u32);
        }
    }
    println!("{}", sum);
    Ok(())
}

// 3768677183
// 19289 ms
async fn async_read8k(path: &str) -> Result<()> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut sum: u32 = 0;
    let mut block = vec![0u8; 1024 * 8];
    while file.read_exact(&mut block).await.is_ok() {
        for i in &block {
            sum = sum.wrapping_add(*i as u32);
        }
    }
    println!("{}", sum);
    Ok(())
}

// 3768677183
// 19498 ms
fn read128k(path: &str) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut sum: u32 = 0;
    let mut block = vec![0u8; 4096];
    while file.read_exact(&mut block).is_ok() {
        for i in &block {
            sum = sum.wrapping_add(*i as u32);
        }
    }
    println!("{}", sum);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tokio::task::spawn_blocking(|| {
        println!("started");
        read128k("/home/stock/data/sorted2.bin").await?;
        println!("finished");
    })
    .await?;
    // let mut file = tokio::fs::File::open("/home/stock/data/sorted2.bin").await?;
    // let now = Instant::now();
    // let a = file.read_u32_le().await?;
    // println!("{} {} ns", a, now.elapsed().as_nanos());
    return Ok(());
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open("/tmp/file")?;
    let mut v = vec![255u8; 1024];
    file.write_all(&v)?;
    file.set_len(1024)?;
    let mut mmap = unsafe { MmapOptions::new().len(1024).map_mut(&file)? };
    mmap.as_mut()
        .iter_mut()
        .enumerate()
        .for_each(|(i, x)| *x = (i & 255) as u8);
    let mut mmap = unsafe { MmapOptions::new().len(512).map(&file)? };
    println!("{:?}", mmap.as_ref());
    println!("{:#?}", mmap.as_ref().len());
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&v)?;
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut v)?;
    println!("{:?}", v);
    // mmap.flush()?;

    return Ok(());
    unsafe {
        config::load("config.yaml")?;
    }
    // #![feature(thread_id_value)]
    use rayon::prelude::*;
    std::env::set_var("RAYON_NUM_THREADS", "4");
    let sum: u32 = vec![1u32; 1024]
        .into_par_iter()
        .enumerate()
        .inspect(|x| print!("({} {})", std::thread::current().id().as_u64(), x.0))
        .fold(|| 0u32, |acc, x| acc + x.1)
        .sum();
    debug!("{:#?}", sum);
    return Ok(());
    // memmap release
    //3795679572
    // 58565 ms
    // memmap prepopulate
    // 113229 ms
    let now = Instant::now();
    // let mut file = std::fs::File::open("/home/stock/ssd/sorted")?;
    // let mmap = unsafe { MmapOptions::new().populate().map(&file)? };
    // let mut sum: u32 = 0;
    // for i in mmap.as_ref() {
    //     sum = sum.wrapping_add(*i as u32);
    // }
    // map_mt("/home/stock/ssd/sorted2.bin")?;
    // map("/home/stock/data/sorted2.bin")?;
    // async_read8k("/home/stock/ssd/sorted2.bin").await?;
    println!("{} ms", now.elapsed().as_millis());
    return Ok(());

    // 232.9
    // 65000.0
    // f32: 0.49301612

    // let mut value = f16::from_f32(1.);
    // let mut mul = f16::from_f32(1. + 0.000008333333333 * 234.75);
    // for _ in 0..4000 {
    //     value = f16::from_f32(value.to_f32() * mul.to_f32());
    //     println!("{}", value);
    // }
    // println!("{}", value);
    // let instant = Instant::now();
    // let mut n = f16::from_f32(0.25 - f16::MIN_POSITIVE_SUBNORMAL.to_f32());
    // println!("{}", n);
    // for _ in 0..100 {
    //     n = f16::from_f32(n.to_f32() + f16::MIN_POSITIVE_SUBNORMAL.to_f32());
    //     println!("{}", n);
    // }
    // return Ok(());
    let mut candles = Candles::read_fast(
        "/home/stock/data/Documents/Projects/the_matrix/data_miner/BitMEX/candles/XBTUSD-original.\
         zstd",
    )
    .await?;
    let mut prev = candles.close[0];
    dbg!(prev);
    let tick_size = 0.01;
    let shift = 13u32;
    // let min_val = 1. / (1 << shift) as f32 + 1. / (1 << (shift + 1)) as f32;
    // let rel_move = tick_size / 232.9;
    // let min = min_val / rel_move;
    let min = 1. / (1 << shift) as f32 + 1. / (1 << (shift + 1)) as f32;
    let min = 1.;

    // candles.close[0] = f16::from_f32(candles.close[0] / 232.9 * min).to_f32();
    // for i in 1..candles.close.len() {
    //     let cur = candles.close[i];
    //     candles.close[i] = candles.close[i - 1] * cur / prev;
    //     prev = cur;
    // }
    // let mut max = 0.;
    // for price in &candles.high {
    //     max.max_mut(*price);
    // }
    // dbg!(max);
    // panic!();

    let mut balance = f16::from_f32(1.);
    let mut position = f16::from_f32(0.0f32);

    for i in (0..candles.close.len()).step_by(10).take(200) {
        // let price = (candles.close[i] - 232.9) / (65000. - 232.9) + f32::min_positive_value();
        let price = candles.close[i];
        if position == f16::from_f32(0.) {
            let b = balance.to_f32();
            let fee = f16::from_f32(b * 0.00075).to_f32();
            let fee = 0.;
            balance = f16::from_f32(b - fee);
            position = f16::from_f32(balance.to_f32() / price);
        } else {
            let p = position.to_f32();
            let value = f16::from_f32(p * price).to_f32();
            let fee = f16::from_f32(value * (0.00075)).to_f32();
            let fee = 0.;
            balance = f16::from_f32(value - fee);
            position = f16::from_f32(0.);
        }
        // dbg!(balance, price);
    }
    balance = f16::from_f32(
        balance.to_f32() - f16::from_f32(balance.to_f32() * 0.0009765625 * 200.).to_f32(),
    );
    dbg!(balance);

    let mut balance = 1.0f32;
    let mut position = 0.0f32;
    //
    for i in (0..candles.close.len()).step_by(10).take(200) {
        // let price = (candles.close[i] - 232.9) / (65000. - 232.9) + f32::min_positive_value();
        let price = candles.close[i];
        if position == 0. {
            let fee = balance * 0.00075;
            let fee = 0.;
            balance = balance - fee;
            position = balance / price;
        } else {
            let fee = (position * price * (0.00075));
            let fee = 0.;
            balance = position * price - fee;
            dbg!(balance);
            position = 0.;
        }
        // dbg!(balance, price);
    }
    balance = balance - balance * (0.00075 * 200.);
    dbg!(balance);
    //
    // println!("{}", instant.elapsed().as_millis());
    // let sri = cacache::write("./my-cache", "my-key", b"hello").await?;
    // find_max("/home/stock/data/Documents/Projects/the_matrix/reports/BitMEX/XBTUSD/tpro/
    // seek_reports.bin").unwrap();
    return Ok(());
    // let instant = Instant::now();
    // use async_compression::{tokio::write::ZstdEncoder, Level};
    // let mut writer =
    // BufWriter::new(tokio::fs::File::create("/home/stock/data/test1.bin").await?);
    // let mut writer = ZstdEncoder::with_quality(writer, Level::Precise(1));
    // for i in 0..u32::MAX {
    //     let data = i as u64;
    //     writer.write(data.as_ne_bytes()).await?;
    // }
    // writer.shutdown().await?;
    // use std::io::Write;
    // let mut writer = {
    //     let target = std::fs::File::create("/home/stock/data/test-5.bin")?;
    //     zstd::Encoder::new(target, -5)?
    // };
    // for i in 0..u32::MAX {
    //     let data = i as u64;
    //     writer.write(data.as_ne_bytes())?;
    // }
    //
    // writer.finish()?;
    // println!("{}s", instant.elapsed().as_secs());
    // let candles =
    // Candles::read("/home/stock/data/Documents/Projects/the_matrix/data_miner/BitMEX/candles/
    // XBTUSD.zstd").await?; let mut close_dif = 0.;
    // let mut low_dif = f32::MAX;
    // let mut high_dif = 0.;
    // for i in 1..candles.len() {
    //     let prev_close = candles.close[i - 1];
    //     close_dif.max_mut((candles.close[i] - prev_close) / prev_close);
    //     low_dif.min_mut((candles.low[i] - prev_close) / prev_close);
    //     high_dif.max_mut((candles.high[i] - prev_close) / prev_close);
    // }
    // println!("{} {} {}", close_dif, low_dif, high_dif);
    // panic!();
    let args = Args::parse();
    unsafe {
        config::load(&args.config)?;
    }
    match args.sub_command {
        SubCommand::Fetch(args) => {
            select_exchange(&args.args.exchange);
            match args.fetch_command {
                FetchCommand::Candles(candles_args) => {
                    fetch_candles(args.args, candles_args).await?;
                }
            }
        }
        SubCommand::Insert(insert_command) => match insert_command {
            InsertCommand::SourceCode(a) => {
                let mut path = PathBuf::from(
                    &CONFIG
                        .cl_src_dir
                        .as_ref()
                        .expect("No models dir specified in config"),
                )
                .join(&a.name);
                path.set_extension("cl");
                let mut file = File::open(path).await?;
                let mut source = String::new();
                file.read_to_string(&mut source).await?;
                insert_model_source(&a.name, &source)?;
            }
            InsertCommand::VariableValues(a) => {
                let source_id = get_model_source_id(&a.name)?;
                let values = Writable::write_to_vec(&a.values)?;
                insert_model_values(source_id, &values)?;
            }
        },
    }

    Ok(())
}
