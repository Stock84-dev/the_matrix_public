use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::hint::unreachable_unchecked;
use std::io::{Read, SeekFrom, Write};
use std::path::PathBuf;

use chrono::{DateTime, Datelike, Duration, Timelike};
use config::{get_exchange_config, CONFIG};
// use db::DB;
use futures_util::StreamExt;
use memmap2::{Mmap, MmapOptions};
use merovingian::hlcv::{change_timeframe_src_offset, Hlcv, Hlcvs, MappedHlcvs};
use mouse::definitions::UnsafeSyncRefCell;
use mouse::error::Result;
use mouse::ext::{
    PathExt, SizeOfVal, StaticSize, Transmutations, Uninitialized, UninitializedCollection,
};
use mouse::helpers::{
    object_as_slice, object_as_slice_mut, open_rwc_all, open_rwc_async, ptr_as_slice,
    ptr_as_slice_mut,
};
use mouse::num::rust_decimal::prelude::ToPrimitive;
use mouse::num::NumExt;
use mouse::prelude::*;
use mouse::time::{IntoDateTime, Timestamp};
use nebuchadnezzar::core::paginators::WhileSuperPaginator;
use nebuchadnezzar::core::requests::TradesGetRequest;
use nebuchadnezzar::core::Credentials;
use tokio::fs::File;
use tokio::io;
use tokio::io::{
    AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
};

const BLOCK_SIZE: usize = 128 * 1024;

#[repr(C)]
#[derive(Debug)]
struct HlcvHeader {
    // order is important
    start_ts: u32,
    end_timestamp_s: u32,
}

// BitMEX/hlcv/XBTUSD.bin

struct MissingBlock {
    start_ts: i64,
    end_ts: i64,
}

pub async fn load_hlcv(
    exchange: &str,
    market: &str,
    start_ts: u32,
    count: usize,
) -> Result<MappedHlcvs> {
    //    let path = path!(exchange, "hlcv", market);
    //    let file_config = DB.get_or_create_file(&path).await?;
    //    if file_config.host_path.is_empty() {
    //        unimplemented!("Only local file paths are implemented");
    //    }
    //    let end_ts = start_ts as i64 + count as i64;
    //    let mut blocks = DB
    //        .load_hlcv_blocks(file_config.file_id, start_ts as i64, end_ts)
    //        .await?;
    //    blocks.sort_by_key(|x| x.start_time);
    //    let mut blocks_to_load = Vec::new();
    //    let mut missing_blocks = Vec::new();
    //    let mut i_ts = start_ts as i64;
    //    for (i, block) in blocks.iter().enumerate() {
    //        if i_ts >= block.start_time.timestamp() && i_ts < block.end_time.timestamp() {
    //            i_ts = block.end_time.timestamp();
    //            blocks_to_load.push(block.clone());
    //        } else if i_ts < block.start_time.timestamp() {
    //            let prev_block = &blocks[i - 1];
    //            missing_blocks.push()
    //        } else {
    //        }
    //    }

    //    use std::path::PathBuf;
    //    use std::str::FromStr;
    //    let a = dbg!(PathBuf::from_str(
    //        r#"C:\Documents\Newsletters\Summer2018.pdf"#
    //    ))
    //    .unwrap();
    //    for b in a.iter() {
    //        dbg!(b);
    //    }
    //    let b = dbg!(PathBuf::from_str("/Documents/Newsletters/Summer2018.pdf")).unwrap();
    //    for b in b.iter() {
    //        dbg!(b);
    //    }
    info!("Loading hlcv...");
    let hlcv;
    let mut path = CONFIG.cache_dir.join(exchange).join("hlcv").join(market);
    path.set_extension("bin");
    let timeframe = 1;
    let end_ts = start_ts + count as u32 * timeframe;
    debug!(
        "request start {}, request end {}",
        start_ts.into_date_time(),
        end_ts.into_date_time()
    );
    if path.exists_async().await {
        let mut file_reader = BufReader::with_capacity(BLOCK_SIZE, open_rwc_async(&path).await?);
        let mut header = unsafe { HlcvHeader::uninitialized_unsafe() };
        let header_slice = unsafe { object_as_slice_mut(&mut header, HlcvHeader::size()) };
        file_reader.read_exact(header_slice).await?;
        debug!(
            "saved start: {}, saved end: {}",
            header.start_ts.into_date_time(),
            header.end_timestamp_s.into_date_time()
        );
        let mut file_writer;
        if end_ts <= header.start_ts {
            let mut prepend_path = CONFIG
                .cache_dir
                .join(exchange)
                .join("hlcv")
                .join(format!("{}{}", market, "_prepend"));
            prepend_path.set_extension("tmp");
            let mut tmp =
                BufWriter::with_capacity(BLOCK_SIZE, open_rwc_async(&prepend_path).await?);
            tmp.seek(SeekFrom::Start(HlcvHeader::size() as u64)).await?;
            append_hlcv(
                exchange,
                market,
                start_ts,
                header.start_ts,
                &mut tmp,
                &mut header.start_ts,
            )
            .await?;
            tmp.seek(SeekFrom::Start(0)).await?;
            tmp.write_all(header_slice).await?;
            io::copy(&mut file_reader, &mut tmp).await?;
            file_reader.shutdown().await?;
            // close file to allow removing it
            drop(file_reader);
            file_writer = tmp;
            tokio::fs::remove_file(&path).await?;
            tokio::fs::rename(&prepend_path, &path).await?;
        } else {
            file_reader.shutdown().await?;
            file_writer = BufWriter::with_capacity(BLOCK_SIZE, file_reader.into_inner());
        }

        if end_ts > header.end_timestamp_s {
            // prevent changing header
            let mut not_used = 0;
            file_writer.seek(SeekFrom::End(0)).await?;
            append_hlcv(
                exchange,
                market,
                header.end_timestamp_s,
                end_ts,
                &mut file_writer,
                &mut not_used,
            )
            .await?;
            header.end_timestamp_s = end_ts;
            file_writer.seek(SeekFrom::Start(0)).await?;
            file_writer.write_all(header_slice).await?;
            file_writer.shutdown().await?;
        }
        let file = file_writer.into_inner();
        hlcv = read_hlcv(start_ts, end_ts, file, &header).await?;
    } else {
        hlcv = load_hlcv_no_file(exchange, market, start_ts, end_ts, path).await?;
    }
    info!("Loading hlcv...DONE");

    Ok(hlcv)
}

pub async fn load_hlcv_no_file(
    exchange: &str,
    market: &str,
    start_timestamp_s: u32,
    end_ts: u32,
    path: PathBuf,
) -> Result<MappedHlcvs> {
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let mut file = BufWriter::new(open_rwc_async(&path).await?);
    file.seek(SeekFrom::Start(8)).await?;
    let mut actual_start_ts = 0;
    append_hlcv(
        exchange,
        market,
        start_timestamp_s,
        end_ts,
        &mut file,
        &mut actual_start_ts,
    )
    .await?;
    file.seek(SeekFrom::Start(0)).await?;
    let header = HlcvHeader {
        start_ts: actual_start_ts,
        end_timestamp_s: end_ts,
    };
    file.write_all(header.as_u8_slice()).await?;
    file.shutdown().await?;
    read_hlcv(actual_start_ts, end_ts, file.into_inner(), &header).await
}

async fn read_hlcv(
    start_ts: u32,
    end_ts: u32,
    file: File,
    header: &HlcvHeader,
) -> Result<MappedHlcvs> {
    let offset_len = if start_ts > header.start_ts {
        start_ts as u64 - header.start_ts as u64
    } else {
        0
    };
    let offset = HlcvHeader::size() as u64 + offset_len * Hlcv::size() as u64;
    let file = file.into_std().await;
    let src_len = (end_ts - start_ts) as usize;
    let mmap = unsafe {
        MmapOptions::new()
            .offset(offset)
            .len(src_len * Hlcv::size())
            .map(&file)?
    };
    Ok(MappedHlcvs {
        map: mmap,
        start_ts,
    })
}

/// fetched candles may not start at start_ts if exchange doesn't have data
/// it will load first available candle after start_ts
async fn append_hlcv(
    exchange_name: &str,
    market: &str,
    start_ts: u32,
    end_ts: u32,
    writer: &mut (impl AsyncWrite + AsyncSeek + Unpin),
    first_trade_ts: &mut u32,
) -> Result<()> {
    // trace!("write");
    // let total_ts = (end_ts - start_ts) as f32;
    let mut tmp_path = CONFIG
        .cache_dir
        .join(exchange_name)
        .join("hlcv")
        .join(market);
    tmp_path.set_extension("tmp");
    info!("Fetching public trades...");
    let mut tmp = BufWriter::new(open_rwc_async(&tmp_path).await?);
    // set file length just in case if there was process tremination and file was left
    // if the file is bigger than current requested file then we have copied what we shouldn't
    tmp.get_mut().set_len(0).await?;

    let mut client = nebuchadnezzar::exchanges()
        .into_iter()
        .find(|x| x.name() == exchange_name)
        .expect("Valid exchange name")
        .new_client_dyn();
    if let Some(config) = get_exchange_config() {
        if !config.api_key.is_empty() {
            info!("Authenticating...");
            client.authenticate(Credentials::new(&config.api_key, &config.api_secret))?;
        }
    }
    let mut date_time = start_ts.into_date_time();
    let mut offset = 0;
    // Safety: do not spawn a task
    let end_date_time = end_ts.into_date_time();
    let paginator = Box::pin(WhileSuperPaginator::new(
        Ok(TradesGetRequest {
            symbol: market.to_string(),
            count: Some(1),
            offset: None,
            start_time: Some(date_time),
            end_time: None,
        }),
        |result, max_count| match result {
            Ok(response) => {
                if response.is_empty() {
                    None
                } else {
                    // info!("{}%", (end_ts - date_time.timestamp_s()) as f32 / total_ts);
                    let last_ts = response.last().unwrap().timestamp;
                    if last_ts >= *end_date_time.borrow() {
                        // debug!("{:#?}", response);
                        return None;
                    }
                    if last_ts != date_time {
                        date_time = last_ts;
                        offset = response
                            .iter()
                            .rev()
                            .filter(|x| x.timestamp == date_time)
                            .count() as i32;
                    } else {
                        offset += response.len() as i32;
                    }
                    Some(Ok(TradesGetRequest {
                        symbol: market.to_string(),
                        count: Some(max_count),
                        offset: Some(offset),
                        start_time: Some(date_time),
                        end_time: None,
                    }))
                }
            }
            Err(_) => None,
        },
    ));
    let mut stream = client.paginate_trades(paginator);
    let mut trades = stream.next().await.unwrap()?.into_iter();
    let mut first_trade = trades.next().unwrap();
    let mut hlcv = Hlcv {
        high: first_trade.price.to_f32().unwrap(),
        low: first_trade.price.to_f32().unwrap(),
        close: first_trade.price.to_f32().unwrap(),
        volume: first_trade.amount.to_f32().unwrap(),
    };
    let slice = unsafe { object_as_slice(&hlcv, hlcv.size_of_val()) };
    if first_trade.timestamp.nanosecond() != 0 {
        first_trade.timestamp = first_trade
            .timestamp
            .with_nanosecond(0)
            .unwrap()
            .with_second(1)
            .unwrap();
    }
    // debug!("{:#?}", end_date_time.borrow());
    let mut ts = first_trade.timestamp;
    loop {
        match trades.next() {
            None => match stream.next().await {
                None => break,
                Some(result) => trades = result?.into_iter(),
            },
            Some(trade) if trade.timestamp > ts => {
                // debug!("{:#?}", trade);
                let mut duration = trade.timestamp - ts;
                if duration.num_milliseconds() % 1000 != 0 {
                    duration = duration + Duration::seconds(1)
                }
                // trace!("filling {}", duration.num_seconds() + 1);
                // filling gapse between hlcv
                tmp.write_all(slice).await?;
                hlcv.volume = 0.;
                for _ in 1..duration.num_seconds() + 1 {
                    // trace!("executed");
                    // zstd with level 4 is the fastest
                    tmp.write_all(slice).await?;
                }
                if trade.timestamp >= *end_date_time.borrow() {
                    break;
                }
                hlcv.high = trade.price.to_f32().unwrap();
                hlcv.low = trade.price.to_f32().unwrap();
                hlcv.close = trade.price.to_f32().unwrap();
                hlcv.volume = trade.amount.to_f32().unwrap();
                ts = ts + duration;
            }
            Some(trade) if trade.timestamp <= ts => {
                // trace!("constructing");
                hlcv.close = trade.price.to_f32().unwrap();
                hlcv.high.max_mut(hlcv.close);
                hlcv.low.min_mut(hlcv.close);
                hlcv.volume += trade.amount.to_f32().unwrap();
            }
            _ => unsafe { unreachable_unchecked() },
        }
    }
    tmp.shutdown().await?;
    // trace!("executed");
    let mut tmp = tmp.into_inner();

    tmp.seek(SeekFrom::Start(0)).await?;
    let mut tmp = BufReader::with_capacity(BLOCK_SIZE, tmp);
    io::copy(&mut tmp, writer).await?;
    drop(tmp);
    tokio::fs::remove_file(&tmp_path).await?;
    writer.seek(SeekFrom::Start(4)).await?;
    writer.write_all(ts.timestamp_s().as_u8_slice()).await?;
    *first_trade_ts = first_trade.timestamp.timestamp_s();
    Ok(())
}
