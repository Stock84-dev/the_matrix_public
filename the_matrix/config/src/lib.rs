use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, fmt};

use chrono::DateTime;
use merovingian::variable::Variable;
use mouse::error::{Result, ResultCtxExt};
use mouse::log::*;
use mouse::macros::futures_util::FutureExt;
use mouse::time::Timestamp;
use serde::de::{DeserializeOwned, MapAccess, Visitor};
use serde::{de, Deserialize, Deserializer, Serialize};

lazy_static::lazy_static! {
    pub static ref CONFIG: SuperConfig = Default::default();
}

/// Safety: Race condition possible.
pub unsafe fn load_static<P, C, S>(_path: P, _static_config: &S) -> Result<()>
where
    P: AsRef<Path> + Send + Sync,
    C: DeserializeOwned,
{
    Ok(())
}

/// todo: load this on current thread runtime then spawn_blocking other thread in main this will
/// reduce compile times, it will also ensure that runtime would not deadlock
pub unsafe fn load<P: AsRef<Path> + Send + Sync>(path: P) -> Result<()> {
    env::set_var("RUST_BACKTRACE", "full");
    // initialize tokio handle so that it can be shared between threads later
    //    mouse::ext::set_tokio_handle(tokio::runtime::Handle::current());
    //    set_tokio_handle();
    //    std::future::ready(()).block();
    //    Ok(
    //        *(&*CONFIG as *const _ as *mut SuperConfig) = serde_yaml::from_reader(
    //            File::open(path.as_ref())
    //                .with_context(|| path.as_ref().to_str().unwrap().to_string())?,
    //        )?,
    //    )
    let path = path.as_ref();
    // NOTE: adding anyhow::context here crashes when debugging
    let file = File::open(path)?; //.with_context(|| format!("{}", path.display()))?;
    *(&*CONFIG as *const _ as *mut SuperConfig) = serde_yaml::from_reader(file)?;
    Ok(())
}

pub fn data_file_path(
    exchnage_name: &str,
    relative_path: impl AsRef<Path>,
    extension: &str,
) -> PathBuf {
    let mut path = CONFIG
        .data_dir
        .join(exchnage_name)
        .join(relative_path.as_ref());
    path.set_extension(extension);
    path
}

pub fn select_exchange(exchange_name: &str) {
    unsafe {
        let exchanges = &mut *(&CONFIG.exchanges as *const _ as *mut Vec<ExchangeConfig>);
        for config in exchanges.iter_mut() {
            if config.name == exchange_name {
                config.selected = Some(());
                break;
            }
        }
    }
}

pub fn get_exchange_config() -> Option<&'static ExchangeConfig> {
    CONFIG.exchanges.iter().find(|x| x.selected.is_some())
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SuperConfig {
    pub db: String,
    pub cl_src_dir: Option<String>,
    #[serde(deserialize_with = "deserialize_path_buf")]
    pub data_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path_buf")]
    pub cache_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path_buf")]
    pub reports_dir: PathBuf,
    #[serde(deserialize_with = "deserialize_path_buf")]
    pub report_template_dir: PathBuf,
    pub exchanges: Vec<ExchangeConfig>,
    //    pub construct: ConstructConfig,
    pub iaas: Option<Iaas>,
    // No need to store log configs, so we use custom deserializer that configures logging.
    #[serde(deserialize_with = "deserialize_log_configs")]
    pub logs: (),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Iaas {
    pub storage_account: String,
    pub storage_key: String,
    pub the_matrix_db_url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConstructConfig {
    pub models: Vec<ConstructModelConfig>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConstructModelConfig {
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub variables: Vec<Variable>,
}

impl ConstructModelConfig {
    pub fn start_timestamp_s(&self) -> Result<u32> {
        Ok(DateTime::parse_from_rfc3339(&self.start_time)
            .with_context(|| self.start_time.clone())?
            .timestamp_s())
    }
    pub fn end_timestamp_s(&self) -> Result<u32> {
        Ok(DateTime::parse_from_rfc3339(&self.end_time)
            .with_context(|| self.end_time.clone())?
            .timestamp_s())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub use_testnet: bool,
    pub use_public_data_miner: bool,
    pub api_key: String,
    pub api_secret: String,
    pub max_leverage: f32,
    pub max_orders_per_m: f32,
    pub models: Vec<ModelConfig>,
    pub selected: Option<()>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub market: String,
    pub target_leverage: f32,
    pub variable_values: Vec<f32>,
}

#[cfg(test)]
fn deserialize_log_configs<'de, D>(_deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    Ok(())
}

#[cfg(not(test))]
fn deserialize_log_configs<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    struct InnerVisitor;

    impl<'de> Visitor<'de> for InnerVisitor {
        type Value = ();

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a struct containing yaml data")
        }

        fn visit_map<V>(self, mut map: V) -> Result<(), V::Error>
        where
            V: MapAccess<'de>,
        {
            let program = std::env::current_exe()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            if program == "default" {
                panic!("Program name cannot be 'default'.")
            }
            let mut config = None;
            let mut default_conf = true;
            while let Some((key, value)) = map.next_entry::<String, LogConfig>()? {
                if program == key {
                    config = Some(value);
                    default_conf = false;
                    break;
                } else if key == "default" {
                    config = Some(value);
                }
            }
            match config {
                None => {
                    println!("No default log configuration specified.");
                }
                Some(c) => {
                    if default_conf {
                        println!("Warning: Using default log configuration.");
                    }
                    c.configure().unwrap();
                }
            }
            Ok(())
        }
    }

    deserializer.deserialize_struct("", LogConfigs::NAMES, InnerVisitor)
}

mouse::field_names! {
    #[derive(Deserialize, Serialize)]
    pub struct LogConfigs {
        #[serde(rename = "default")]
        all: LogConfig,
        matrix: Option<LogConfig>,
        construct: Option<LogConfig>,
        morpheus: Option<LogConfig>,
        mainframe: Option<LogConfig>,
        the_architect: Option<LogConfig>,
    }
}

fn deserialize_path_buf<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct InnerVisitor;

    impl<'de> de::Visitor<'de> for InnerVisitor {
        type Value = PathBuf;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing path data")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let string: String = serde_yaml::from_str(v).map_err(E::custom)?;
            Ok(PathBuf::from(string))
        }
    }

    deserializer.deserialize_str(InnerVisitor)
}
