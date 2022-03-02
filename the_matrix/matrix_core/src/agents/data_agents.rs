use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use async_compression::tokio::{bufread, write};
use async_compression::Level;
use async_trait::async_trait;
use chrono::Utc;
use iaas::azure::archive::archive;
use iaas::mysql::insert_maintenance_snapshot;
use merovingian::minable_models::*;
use merovingian::speedy::{LittleEndian, Writable};
use mouse::error::Result;
use mouse::log::*;
use mouse::time::Timestamp;
use path_slash::PathExt;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter, SeekFrom};

use crate::agents::network_agents::{ExchangeListener, InstrumentConfig};

pub struct ExchangeDataAgent {
    data_miner: DataMiner,
}

impl ExchangeDataAgent {
    pub fn new(data_path: impl AsRef<Path>, exchange_name: impl AsRef<Path>) -> ExchangeDataAgent {
        let mut path = PathBuf::new();
        path.push(data_path);
        path.push(exchange_name);
        ExchangeDataAgent {
            data_miner: DataMiner::new(path),
        }
    }
}

macro_rules! impl_fn {
    ($fn_name:ident, $arg0_type:ty, $file_name:expr) => {
        fn $fn_name<'life0, 'life1, 'async_trait>(
            &'life0 mut self,
            arg0: &'life1 $arg0_type,
        ) -> Pin<Box<dyn core::future::Future<Output = Result<()>> + Send + 'async_trait>>
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait,
        {
            let future = async move { self.data_miner.write($file_name, arg0).await };

            Box::pin(future)
        }
    };

    ($fn_name:ident, $arg0_type:ty, $arg1_type:ty, $file_name:expr) => {
        fn $fn_name<'life0, 'life1, 'life2, 'async_trait>(
            &'life0 mut self,
            arg0: &'life1 $arg0_type,
            arg1: &'life2 $arg1_type,
        ) -> Pin<Box<dyn core::future::Future<Output = Result<()>> + Send + 'async_trait>>
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            'life2: 'async_trait,
            Self: 'async_trait,
        {
            let path = Path::new($file_name).join(arg1);
            let future = async move { self.data_miner.write(path, arg0).await };

            Box::pin(future)
        }
    };
}

#[async_trait]
impl ExchangeListener for ExchangeDataAgent {
    impl_fn!(on_announcement, Announcement, "announcements");
    impl_fn!(on_public_trade, Trade, String, "public_trades");
    impl_fn!(on_chat_message, ChatMessage, "chat_messages");
    impl_fn!(on_connected_users_changed, Connected, "connected");
    impl_fn!(on_funding, Funding, String, "funding");

    async fn on_instrument_changed(
        &mut self,
        instrument: &Instrument,
        symbol: &String,
        _: &Option<&InstrumentConfig>,
    ) -> Result<()> {
        let path = Path::new("instruments").join(symbol);
        self.data_miner.write(path, instrument).await
    }

    impl_fn!(on_insurance_updated, Insurance, String, "insurances");
    impl_fn!(
        on_public_liquidation,
        PublicLiquidation,
        String,
        "public_liquidations"
    );
    impl_fn!(
        on_order_book_updated,
        OrderBookUpdate,
        String,
        "order_books"
    );
    impl_fn!(on_margin_changed, Margin, "margin");
    impl_fn!(on_position_changed, Position, "positions");
    impl_fn!(on_maintenance, Maintenance, "service_status");

    async fn on_shutdown(&mut self) -> Result<()> {
        self.data_miner.shutdown().await
    }
}

#[allow(dead_code)]
pub fn read_from_binary_file<T: for<'de> serde::de::Deserialize<'de>>(
    path: &String,
) -> Result<Vec<T>> {
    let file = std::fs::File::open(path)?;
    let mut buf_reader = std::io::BufReader::new(file);
    let mut data = Vec::new();
    loop {
        let datum: T = match bincode::deserialize_from(&mut buf_reader) {
            Ok(t) => t,
            Err(e) => match *e {
                bincode::ErrorKind::Io(io_err)
                    if io_err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                _ => {
                    error!("Error while deserializing: {:?}", e);
                    panic!("Error while deserializing: {:?}", e);
                }
            },
        };
        data.push(datum);
    }
    return Ok(data);
}

#[derive(Debug)]
pub struct DataMiner {
    streams: HashMap<PathBuf, Writer>,
    data_path: PathBuf,
    // Ensuring that at least code panics when BufWriter isn't flushed as we will lose data.
    is_shut_down: bool,
}

impl DataMiner {
    pub fn new(data_path: impl AsRef<Path>) -> DataMiner {
        DataMiner {
            streams: Default::default(),
            data_path: data_path.as_ref().into(),
            is_shut_down: false,
        }
    }

    #[cfg(feature = "test")]
    pub async fn write<T>(&mut self, _relative_path: impl AsRef<Path>, _data: &T) -> Result<()>
    where
        T: Writable<LittleEndian>,
    {
        Ok(())
    }

    #[cfg(not(feature = "test"))]
    pub async fn write<T>(&mut self, relative_path: impl AsRef<Path>, data: &T) -> Result<()>
    where
        T: Writable<LittleEndian>,
    {
        let relative_path = relative_path.as_ref();
        match self.streams.get_mut(relative_path) {
            None => {
                let mut writer = Writer::new();
                let result = writer.write(&self.data_path, relative_path, data).await;
                let relative_path = PathBuf::from(&relative_path);
                self.streams.insert(relative_path, writer);
                result?;
            }
            Some(writer) => {
                writer.write(&self.data_path, relative_path, data).await?;
            }
        }
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.is_shut_down = true;
        let data_path = &self.data_path;
        let futs = self
            .streams
            .iter_mut()
            .map(|(relative_path, writer)| writer.flush(data_path, relative_path));

        futures::future::try_join_all(futs).await?;
        info!(
            "Successfully shut down data miner for '{}'",
            self.data_path.to_str().unwrap()
        );
        Ok(())
    }
}

impl Drop for DataMiner {
    fn drop(&mut self) {
        if !self.is_shut_down {
            error!("Data miner dropped without being shut down.");
        }
    }
}

#[derive(Debug)]
struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Writer {
        Writer { buf: vec![] }
    }

    async fn write<T: Writable<LittleEndian>>(
        &mut self,
        data_path: &PathBuf,
        relative_path: impl AsRef<Path>,
        data: &T,
    ) -> Result<()> {
        data.write_to_stream(&mut self.buf)?;
        if self.buf.len() < 8196 {
            return Ok(());
        }
        let relative_path = relative_path.as_ref();
        let mut path = data_path.join(&relative_path);
        path.set_extension("lzma");
        create_sub_dirs(&path).await?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .await?;
        let size_hint = file.seek(SeekFrom::End(0)).await? as usize + self.buf.len() / 2;
        let mut file = self.write_to_file(file).await?;
        let rand_10p_usage = rand::random::<u32>() as f32 / u32::MAX as f32 / 10.;
        let usage = fs3::free_space(&path)? as f32 / fs3::total_space(&path)? as f32;
        if usage + rand_10p_usage < 1. {
            return Ok(());
        }
        file.seek(SeekFrom::Start(0)).await?;
        let mut reader = bufread::LzmaDecoder::new(BufReader::new(file));
        reader.multiple_members(true);
        let mut reader =
            bufread::LzmaEncoder::with_quality(BufReader::new(reader), Level::Precise(4));
        let blob_name = format!(
            "{}/{}-{}.lzma",
            data_path.file_name().unwrap().to_str().unwrap(),
            relative_path.to_slash().unwrap(),
            Utc::now().timestamp_s()
        );
        archive(&blob_name, &mut reader, size_hint).await?;
        let file = reader.into_inner().into_inner().into_inner().into_inner();
        file.set_len(0).await?;

        Ok(())
    }

    async fn flush(&mut self, data_path: &PathBuf, relative_path: &Path) -> Result<()> {
        let mut path = data_path.join(relative_path);
        path.set_extension("lzma");
        create_sub_dirs(&path).await?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        self.write_to_file(file).await?;
        Ok(())
    }

    async fn write_to_file(&mut self, file: File) -> Result<File> {
        let mut writer = write::LzmaEncoder::with_quality(BufWriter::new(file), Level::Precise(4));
        let result1 = writer.write(&self.buf).await;
        let result2 = writer.shutdown().await;
        self.buf.clear();
        result1?;
        result2?;
        Ok(writer.into_inner().into_inner())
    }
}

async fn create_sub_dirs(path: &Path) -> Result<()> {
    let path = path.parent().unwrap();
    if tokio::fs::metadata(path).await.is_err() {
        tokio::fs::create_dir_all(path).await?;
    }
    Ok(())
}

pub struct DbAgent {
    exchange_id: u16,
}

impl DbAgent {
    pub fn new(exchange_id: u16) -> DbAgent {
        DbAgent { exchange_id }
    }
}

#[async_trait]
impl ExchangeListener for DbAgent {
    async fn on_maintenance(&mut self, maintenance: &Maintenance) -> Result<()> {
        insert_maintenance_snapshot(&maintenance, self.exchange_id).await?;
        Ok(())
    }
}
