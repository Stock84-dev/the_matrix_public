#![deny(unused_must_use)]

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use chrono::DateTime;
use clap::Clap;
use config::{get_exchange_config, select_exchange, CONFIG};
use construct_core::research_model;
use construct_core::research_model::Model;
use half::f16;
use iaas::mysql::models::{ExchangeConfig, ModelConfig};
use matrix_core::agents::network_agents::mock_network_agent::MockNetworkAgent;
use merovingian::candles::Candles;
use merovingian::non_minable_models::CLFlags;
use merovingian::speedy::Writable;
use merovingian::variable;
use merovingian::variable::Variable;
use mouse::error::*;
use mouse::log::*;
use mouse::num::traits::ToPrimitive;
use mouse::thread_pool;
use mouse::time::Timestamp;
use opencl::{KernelManagerBuilder, TestConfig};
use residual_self_image::seek_report::SeekStat;

#[derive(Clap)]
#[clap(version, about, author)]
struct Args {
    #[clap(long, short, default_value = "config.yaml")]
    /// Path to config file.
    config: String,
    #[clap(long, short)]
    /// Run tests for all models.
    all: bool,
    #[clap(long, short)]
    /// Run test for specific model.
    model: Option<String>,
    #[clap(long, short)]
    /// Exchange name to get candles from.
    exchange: String,
    #[clap(long, short)]
    /// Values of a model to test against.
    values: Option<Vec<f32>>,
}

async fn test_model(model_setup: TestConfig, mut candles: Candles) -> Result<Candles> {
    info!("Running test for {}", model_setup.model_name);
    trace!("Testing construct cache and matrix cache.");

    let (_tx, rx) = multiqueue::broadcast_queue(1);
    // Cloning because model increases variables by 1 when done
    let mut model = Model::new(
        model_setup.model_name.clone(),
        candles,
        model_setup.variables.clone(),
    );
    trace!("Testing backtest and seek models.");
    println!("***********************************************************************************");
    model = model.optimize(
        rx.clone(),
        model_setup.n_iterations,
        model_setup.batch_size as usize,
        CLFlags::TEST | CLFlags::DEBUG,
    )?;
    candles = model.candles;
    info!("Backtest and seek models are the same.");
    println!("***********************************************************************************");
    return Ok(candles);
    let stat_account = research_model::TESTED_STAT_ACCOUNTS
        .lock()
        .unwrap()
        .first()
        .unwrap()
        .clone();
    let exchange_config = ExchangeConfig {
        id: u16::MAX,
        use_testnet: false,
        use_public_data_miner: false,
        api_key: "".to_string(),
        api_secret: "".to_string(),
        max_leverage: 1.0,
        max_orders_per_m: 2.0,
    };
    let variable_values: Vec<_> = model_setup.variables.iter().map(|x| x.value).collect();
    let model_configs = vec![ModelConfig {
        market_model_id: u32::MAX,
        market: candles.market.clone(),
        target_leverage: 1.0,
        model_source_id: 0,
        serialized_variable_values: variable_values.write_to_vec().unwrap(),
    }];
    let mut agent = MockNetworkAgent::new(exchange_config, model_configs).await?;
    let model_state = agent.test().await?;
    let matrix_balance = model_state.snapshot.balance.to_f32().unwrap();
    let construct_balance = stat_account.balance;
    let diff = matrix_balance / construct_balance;
    debug!("{:#?}", stat_account);
    debug!("{:#?}", model_state.snapshot);
    if diff > 1.01
        || diff < 0.99
        || stat_account.n_win_trades as u32
            != model_state
                .snapshot
                .position_snapshot
                .position_close_snapshot
                .n_win_trades
        || stat_account.n_loss_trades as u32
            != model_state
                .snapshot
                .position_snapshot
                .position_close_snapshot
                .n_loss_trades
    {
        debug!("{:#?}", stat_account);
        debug!("{:#?}", model_state.snapshot);
        error!("{}.cl ... FAIL", model_setup.model_name);
        return Err(merovingian::error::TestError::TestFailed.into());
    } else {
        info!("{}.cl ... OK", model_setup.model_name)
    }

    Ok(candles)
}

#[tokio::main]
async fn main() -> Result<()> {
    thread_pool::init_global(1, std::usize::MAX);
    let args: Args = Args::parse();
    unsafe {
        config::load(&args.config)?;
    }
    select_exchange(&args.exchange);
    let mut candles = Candles::read(
        CONFIG.data_dir.join(&args.exchange).join("candles").join(
            &get_exchange_config()
                .expect("exchange is required exchange in config")
                .models[0]
                .market,
        ),
    )
    .await?;
    // debug!("{:#?}", candles.timestamp.last());
    // candles.trim(1623024000, *candles.timestamp.last().unwrap());

    // test_tested("/home/stock/data/Documents/Projects/the_matrix/reports/BitMEX/XBTUSD/tpro/
    // seek_reports.bin", candles).await?; return Ok(());
    let mut builder = KernelManagerBuilder::new(0);
    builder.test(true);
    let mut tkm = builder.build_test_kernel_manager()?;
    if args.all {
        match &args.values {
            None => {
                while let Some(result) = tkm.get_next_test_setup() {
                    let model_setup = result?;
                    candles = test_model(model_setup, candles).await?;
                }
            }
            Some(values) => {
                while let Some(result) = tkm.get_next_test_setup() {
                    let model_setup = result?;
                    let config = TestConfig {
                        model_name: model_setup.model_name,
                        batch_size: 1,
                        variables: build_variables(values),
                        n_iterations: 1,
                    };
                    candles = test_model(config, candles).await?;
                }
            }
        }
    } else {
        match args.model {
            Some(model) => match &args.values {
                None => {
                    test_model(tkm.get_test_setup_for(model)?, candles).await?;
                }
                Some(values) => {
                    let config = TestConfig {
                        model_name: model,
                        batch_size: 1,
                        variables: build_variables(values),
                        n_iterations: 1,
                    };
                    test_model(config, candles).await?;
                }
            },
            None => error!("No model specified!"),
        }
    }
    Ok(())
}

async fn test_tested(seek_reports_file: impl AsRef<Path>, mut candles: Candles) -> Result<()> {
    let mut file = BufReader::new(File::open(seek_reports_file.as_ref())?);
    let _: Vec<Variable> = bincode::deserialize_from(&mut file)?;
    let mut combination = 1;
    let mut config = CONFIG
        .construct
        .models
        .iter()
        .find(|x| x.name == "tpro")
        .expect("no model in config")
        .clone();

    candles.trim(
        DateTime::parse_from_rfc3339(&config.start_time)
            .with_context(|| config.start_time.clone())?
            .timestamp_s(),
        DateTime::parse_from_rfc3339(&config.end_time)
            .with_context(|| config.end_time.clone())?
            .timestamp_s(),
    );
    variable::reset(&mut config.variables);
    // debug!("{:#?}", variable::current_combination(&config.variables));

    while let Ok(stat) = bincode::deserialize_from::<_, SeekStat>(&mut file) {
        let max = f16::from_f32(1.4);
        if stat.balance > max {
            variable::increase_to_combination(&mut config.variables, combination);
            let config = TestConfig {
                model_name: "tpro".to_string(),
                batch_size: 1,
                variables: config.variables.clone(),
                n_iterations: 1,
            };
            candles = test_model(config, candles).await?;
        }
        combination += 1;
    }

    Ok(())
}

// Construct doesn't accept variables with values only.
fn build_variables(values: &[f32]) -> Vec<Variable> {
    let mut variables = Vec::with_capacity(values.len());

    for value in values {
        variables.push(Variable::new(*value, value + 1., *value, 1.));
    }
    // Construct doesn't start if current combination is the same as max
    variables.last_mut().unwrap().max += 1.;
    variables
}
