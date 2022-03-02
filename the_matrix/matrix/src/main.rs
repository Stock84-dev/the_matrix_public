#![deny(unused_must_use)]
#![feature(with_options)]
#![feature(thread_id_value)]
#![feature(try_blocks)]
#![feature(async_closure)]
#![feature(nll)]
#![feature(drain_filter)]
#![recursion_limit = "512"]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::Clap;
use config::select_exchange;
use iaas::mysql::load_configs;
use iaas::mysql::models::{ExchangeConfig, ModelConfig};
use matrix_core::agents::network_agents::*;
use matrix_core::agents::{build_and_kill, run_message_loop};
use merovingian::non_minable_models::ExitCode;
use mouse::error::Result;
use mouse::log::*;
use nebuchadnezzar::core::Exchange;
use nebuchadnezzar::exchanges::*;
use tokio::runtime::Runtime;
use tokio::task;

#[derive(Clap)]
#[clap(version, about, author)]
pub struct Args {
    #[clap(long, short, parse(from_os_str), default_value = "config.yaml")]
    /// Path to config file.
    pub config: PathBuf,
    #[clap(long, short)]
    /// Exchange name to use
    pub exchange: String,
}

//"2 or rsi 23 33 54"

/* CONVENTIONS
- Don't use iter().for_each() alone, use it with other iterative functions.
*/

// sync clocks - no need for this pc
// on each minute poll candle price
// update signals
// based on them place market orders
// always place stop loss close on trigger orders first
// when order is placed then place another one
// later on there is probably bulk order place, will suffice for now
// integrade with notify-send when placing orders and when orders are executed
// when orders are executed should come from ws
// remember to cancel stop loss/target order when one is triggered

async fn run_agent<A: NetworkAgent>(
    exchange_config: ExchangeConfig,
    model_configs: Vec<ModelConfig>,
) -> ExitCode {
    let config = exchange_config.clone();

    let task = task::spawn(async move {
        match A::build(exchange_config, model_configs).await {
            Ok(agent) => run_message_loop(Arc::new(tokio::sync::Mutex::new(agent))).await,
            Err(e) => {
                error!("{:?}", e);
                error!("Failed building agent!");
                ExitCode::Fatal
            }
        }
    });
    match task.await {
        Err(e) => {
            error!("{:?}", e);
            error!("Panic caught!");
            trace!("Rebuilding network agent and killing it!");
            let client = A::new_client(config.use_testnet, &config.api_key, &config.api_secret);
            match build_and_kill(&client, config.id, config.use_public_data_miner).await {
                Ok(_) => {
                    info!("Agent killed successfully!");
                    ExitCode::FailedSafely
                }
                Err(e) => {
                    error!("FATAL: Killing agent FAILED. {:#?}.", e);
                    ExitCode::Fatal
                }
            }
        }
        Ok(exit_code) => exit_code,
    }
}

async fn start_trading(
    exchange_name: &str,
    exchange_config: ExchangeConfig,
    model_configs: Vec<ModelConfig>,
) -> ExitCode {
    if Bitmex::new(false).name() == exchange_name {
        run_agent::<BitmexAgent>(exchange_config, model_configs).await
    } else if Bitmex::new(true).name() == exchange_name {
        run_agent::<BitmexAgent>(exchange_config, model_configs).await
    } else {
        error!("Unknown exchange id.");
        ExitCode::Fatal
    }
}

/******************************************************************************************
URGENT
[2021-04-30 00:09:05.636][matrix_core::agents::network_agents::bitmex_agent:391][WARN:5] Connection reset without closing handshake
[2021-04-30 00:09:05.636][matrix_core::agents::network_agents::bitmex_agent:351][ERROR:5] Connection closed!
[2021-04-30 00:09:05.646][matrix_core::agents::network_agent::websocket:123][ERROR:5] Websocket returned None.

Stack backtrace:
   0: <matrix_core::agents::network_agents::bitmex_agent::BitmexAgent as matrix_core::agents::network_agents::NetworkAgent>::handle_message::{{closure}}
   1: <core::future::from_generator::GenFuture<T> as core::future::future::Future>::poll
   2: <core::future::from_generator::GenFuture<T> as core::future::future::Future>::poll
   3: tokio::runtime::task::harness::poll_future
   4: tokio::runtime::task::harness::Harness<T,S>::poll
   5: std::thread::local::LocalKey<T>::with
   6: tokio::runtime::thread_pool::worker::Context::run_task
   7: tokio::runtime::thread_pool::worker::Context::run
   8: tokio::macros::scoped_tls::ScopedKey<T>::set
   9: tokio::runtime::thread_pool::worker::run
  10: tokio::loom::std::unsafe_cell::UnsafeCell<T>::with_mut
  11: <std::panic::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once
  12: tokio::runtime::task::harness::Harness<T,S>::poll
  13: tokio::runtime::blocking::pool::Inner::run
  14: std::sys_common::backtrace::__rust_begin_short_backtrace
  15: core::ops::function::FnOnce::call_once{{vtable.shim}}
  16: <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once
             at /rustc/45b3c28518e4c45dfd12bc2c4400c0d0e9639927/library/alloc/src/boxed.rs:1546:9
      <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once
             at /rustc/45b3c28518e4c45dfd12bc2c4400c0d0e9639927/library/alloc/src/boxed.rs:1546:9
      std::sys::unix::thread::Thread::new::thread_start
             at /rustc/45b3c28518e4c45dfd12bc2c4400c0d0e9639927/library/std/src/sys/unix/thread.rs:71:17
  17: start_thread
  18: clone
******************************************************************************************/

fn main() -> Result<()> {
    // TODO: does price drop/increse when there is funding
    let args = Args::parse();
    unsafe {
        config::load(&args.config)?;
    }
    // panic!("");
    mouse::handlers::setup_ctrlc_handler()?;
    let mut exit_code;
    let rt = Runtime::new().unwrap();
    select_exchange(&args.exchange);
    let configs = load_configs(&args.exchange)?;
    exit_code = rt.block_on(start_trading(&args.exchange, configs.0, configs.1));
    let instant = Instant::now();
    rt.shutdown_timeout(tokio::time::Duration::from_secs(60));
    if instant.elapsed().as_secs() >= 60 {
        error!("Tokio runtime timed out.");
        exit_code = ExitCode::Fatal;
    }
    std::process::exit(exit_code as i32);
}

/*
[2020-04-09 13:57:00][neo::agents::trading_agent:95][DEBUG] Received signal: 5, enter_price: 0
[2020-04-09 13:57:00][neo::agents::trading_agent:119][INFO] Signal::ENTER_BY_BUYING
[2020-04-09 13:57:00][neo::agents::trading_agent:92][TRACE] On new candle
lline: 36, hline: 39, previous_rsi: 44.358067, rsi: 34.76892, length: 95
[2020-04-09 13:57:00][neo::agents::trading_agent:95][DEBUG] Received signal: 5, enter_price: 0
[2020-04-09 13:57:00][neo::agents::trading_agent:119][INFO] Signal::ENTER_BY_BUYING
[2020-04-09 13:57:00][bitmex::client:125][TRACE] Sign message POST/api/v1/order/bulk1586433425{"orders":[{"symbol":"XBTUSD","side":"Buy","simpleOrderQty":null,"orderQty":1,"price":null,"displayQty":null,"stopPx":null,"clOrdID":null,"clOrdLinkID":null,"pegOffsetValue":null,"pegPriceType":null,"ordType":"Market","timeInForce":null,"execInst":null,"contingencyType":null,"text":null},{"symbol":"XBTUSD","side":"Buy","simpleOrderQty":null,"orderQty":1,"price":null,"displayQty":null,"stopPx":null,"clOrdID":null,"clOrdLinkID":null,"pegOffsetValue":null,"pegPriceType":null,"ordType":"Market","timeInForce":null,"execInst":null,"contingencyType":null,"text":null}]}
[2020-04-09 13:57:07][reqwest::async_impl::response:53][DEBUG] Response: '400 Bad Request' for https://www.bitmex.com/api/v1/order/bulk
[2020-04-09 13:57:07][neo::agents::network_agents::bitmex_agent:802][ERROR] Unhandled error: RemoteError { message: "This request has expired - `expires` is in the past. Current time: 1586433426", name: "HTTPError" }

stack backtrace:
   0: failure::backtrace::internal::InternalBacktrace::new
   1: failure::backtrace::Backtrace::new
   2: neo::agents::network_agents::bitmex_agent::BitmexAgent::start_broadcasting_candles::{{closure}}::{{closure}}
   3: tokio::runtime::basic_scheduler::BasicScheduler<P>::block_on
   4: tokio::runtime::context::enter
   5: neo::agents::network_agents::bitmex_agent::BitmexAgent::start_broadcasting_candles::{{closure}}
   6: std::sys_common::backtrace::__rust_begin_short_backtrace
   7: std::panicking::try::do_call
   8: __rust_maybe_catch_panic
             at src/libpanic_unwind/lib.rs:86
   9: core::ops::function::FnOnce::call_once{{vtable.shim}}
  10: <alloc::boxed::Box<F> as core::ops::function::FnOnce<A>>::call_once
             at /rustc/58b834344fc7b9185e7a50db1ff24e5eb07dae5e/src/liballoc/boxed.rs:1016
  11: <alloc::boxed::Box<F> as core::ops::function::FnOnce<A>>::call_once
             at /rustc/58b834344fc7b9185e7a50db1ff24e5eb07dae5e/src/liballoc/boxed.rs:1016
      std::sys_common::thread::start_thread
             at src/libstd/sys_common/thread.rs:13
      std::sys::unix::thread::Thread::new::thread_start
             at src/libstd/sys/unix/thread.rs:80
  12: start_thread
  13: clone

thread '<unnamed>' panicked at 'Unhandled error: RemoteError { message: "This request has expired - `expires` is in the past. Current time: 1586433426", name: "HTTPError" }

stack backtrace:
   0: failure::backtrace::internal::InternalBacktrace::new
   1: failure::backtrace::Backtrace::new
   2: neo::agents::network_agents::bitmex_agent::BitmexAgent::start_broadcasting_candles::{{closure}}::{{closure}}
   3: tokio::runtime::basic_scheduler::BasicScheduler<P>::block_on
   4: tokio::runtime::context::enter
   5: neo::agents::network_agents::bitmex_agent::BitmexAgent::start_broadcasting_candles::{{closure}}
   6: std::sys_common::backtrace::__rust_begin_short_backtrace
   7: std::panicking::try::do_call
   8: __rust_maybe_catch_panic
             at src/libpanic_unwind/lib.rs:86
   9: core::ops::function::FnOnce::call_once{{vtable.shim}}
  10: <alloc::boxed::Box<F> as core::ops::function::FnOnce<A>>::call_once
             at /rustc/58b834344fc7b9185e7a50db1ff24e5eb07dae5e/src/liballoc/boxed.rs:1016
  11: <alloc::boxed::Box<F> as core::ops::function::FnOnce<A>>::call_once
             at /rustc/58b834344fc7b9185e7a50db1ff24e5eb07dae5e/src/liballoc/boxed.rs:1016
      std::sys_common::thread::start_thread
             at src/libstd/sys_common/thread.rs:13
      std::sys::unix::thread::Thread::new::thread_start
             at src/libstd/sys/unix/thread.rs:80
  12: start_thread
  13: clone
', neo/src/agents/network_agents/bitmex_agent.rs:803:29
stack backtrace:
   0: backtrace::backtrace::libunwind::trace
             at /cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.40/src/backtrace/libunwind.rs:88
   1: backtrace::backtrace::trace_unsynchronized
             at /cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.40/src/backtrace/mod.rs:66
   2: std::sys_common::backtrace::_print_fmt
             at src/libstd/sys_common/backtrace.rs:77
   3: <std::sys_common::backtrace::_print::DisplayBacktrace as core::fmt::Display>::fmt
             at src/libstd/sys_common/backtrace.rs:59
   4: core::fmt::write
             at src/libcore/fmt/mod.rs:1052
   5: std::io::Write::write_fmt
             at src/libstd/io/mod.rs:1428
   6: std::sys_common::backtrace::_print
             at src/libstd/sys_common/backtrace.rs:62
   7: std::sys_common::backtrace::print
             at src/libstd/sys_common/backtrace.rs:49
   8: std::panicking::default_hook::{{closure}}
             at src/libstd/panicking.rs:204
   9: std::panicking::default_hook
             at src/libstd/panicking.rs:224
  10: std::panicking::rust_panic_with_hook
             at src/libstd/panicking.rs:470
  11: rust_begin_unwind
             at src/libstd/panicking.rs:378
  12: std::panicking::begin_panic_fmt
             at src/libstd/panicking.rs:332
  13: neo::agents::network_agents::bitmex_agent::BitmexAgent::start_broadcasting_candles::{{closure}}::{{closure}}
  14: tokio::runtime::basic_scheduler::BasicScheduler<P>::block_on
  15: tokio::runtime::context::enter
  16: neo::agents::network_agents::bitmex_agent::BitmexAgent::start_broadcasting_candles::{{closure}}
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.








[2020-04-09 19:11:00][neo::agents::network_agents::bitmex_agent:727][DEBUG] 1586452260 7309.5 7309.5 7301.5 7301.5 1054386
[2020-04-09 19:11:00][neo::agents::trading_agent:92][TRACE] On new candle
lline: 38, hline: 41, previous_rsi: 51.091217, rsi: 50.429493, length: 141
[2020-04-09 19:11:00][neo::agents::trading_agent:95][DEBUG] Received signal: 0, enter_price: 0
[2020-04-09 19:11:00][neo::agents::trading_agent:92][TRACE] On new candle
lline: 37, hline: 39, previous_rsi: 51.87981, rsi: 50.85338, length: 97
[2020-04-09 19:11:00][neo::agents::trading_agent:95][DEBUG] Received signal: 0, enter_price: 0
[2020-04-09 19:11:00][neo::agents::trading_agent:92][TRACE] On new candle
lline: 36, hline: 39, previous_rsi: 51.939304, rsi: 50.886345, length: 95
[2020-04-09 19:11:00][neo::agents::trading_agent:95][DEBUG] Received signal: 0, enter_price: 0
thread '<unnamed>' panicked at 'internal error: entered unreachable code', /home/stock/.cargo/registry/src/github.com-1ecc6299db9ec823/bitmex-0.2.2/src/websocket.rs:102:14
stack backtrace:
   0: backtrace::backtrace::libunwind::trace
             at /cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.40/src/backtrace/libunwind.rs:88
   1: backtrace::backtrace::trace_unsynchronized
             at /cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.40/src/backtrace/mod.rs:66
   2: std::sys_common::backtrace::_print_fmt
             at src/libstd/sys_common/backtrace.rs:77
   3: <std::sys_common::backtrace::_print::DisplayBacktrace as core::fmt::Display>::fmt
             at src/libstd/sys_common/backtrace.rs:59
   4: core::fmt::write
             at src/libcore/fmt/mod.rs:1052
   5: std::io::Write::write_fmt
             at src/libstd/io/mod.rs:1428
   6: std::sys_common::backtrace::_print
             at src/libstd/sys_common/backtrace.rs:62
   7: std::sys_common::backtrace::print
             at src/libstd/sys_common/backtrace.rs:49
   8: std::panicking::default_hook::{{closure}}
             at src/libstd/panicking.rs:204
   9: std::panicking::default_hook
             at src/libstd/panicking.rs:224
  10: std::panicking::rust_panic_with_hook
             at src/libstd/panicking.rs:470
  11: std::panicking::begin_panic
  12: <bitmex::websocket::BitMEXWebsocket as futures_core::stream::Stream>::poll_next
  13: neo::agents::network_agents::bitmex_agent::BitmexAgent::run_message_loop::{{closure}}
  14: <std::future::GenFuture<T> as core::future::future::Future>::poll
  15: tokio::runtime::basic_scheduler::BasicScheduler<P>::block_on
  16: tokio::runtime::context::enter
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.



thread '<unnamed>' panicked at 'called `Option::unwrap()` on a `None` value', neo/src/agents/network_agents/bitmex_agent.rs:446:31
stack backtrace:
   0:     0x5585f0815ba8 - backtrace::backtrace::libunwind::trace::h86edaa2680be3f32
                               at /cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.40/src/backtrace/libunwind.rs:88
   1:     0x5585f0815ba8 - backtrace::backtrace::trace_unsynchronized::h020717321cc60d9f
                               at /cargo/registry/src/github.com-1ecc6299db9ec823/backtrace-0.3.40/src/backtrace/mod.rs:66
   2:     0x5585f0815ba8 - std::sys_common::backtrace::_print_fmt::h95a740d649d8282b
                               at src/libstd/sys_common/backtrace.rs:77
   3:     0x5585f0815ba8 - <std::sys_common::backtrace::_print::DisplayBacktrace as core::fmt::Display>::fmt::h229d12a248a94d4d
                               at src/libstd/sys_common/backtrace.rs:59
   4:     0x5585f083cb0c - core::fmt::write::h7a7c155a9a2fc994
                               at src/libcore/fmt/mod.rs:1052
   5:     0x5585f080ed57 - std::io::Write::write_fmt::hbc3e21ba137de707
                               at src/libstd/io/mod.rs:1428
   6:     0x5585f0818345 - std::sys_common::backtrace::_print::h8ecf04ab6aa60d02
                               at src/libstd/sys_common/backtrace.rs:62
   7:     0x5585f0818345 - std::sys_common::backtrace::print::hbbeb2ccd67fe006e
                               at src/libstd/sys_common/backtrace.rs:49
   8:     0x5585f0818345 - std::panicking::default_hook::{{closure}}::h30799abc567130ac
                               at src/libstd/panicking.rs:204
   9:     0x5585f0818086 - std::panicking::default_hook::h992fc24d479949ec
                               at src/libstd/panicking.rs:224
  10:     0x5585f08189a2 - std::panicking::rust_panic_with_hook::hd5c9bb7319c9d846
                               at src/libstd/panicking.rs:470
  11:     0x5585f081858b - rust_begin_unwind
                               at src/libstd/panicking.rs:378
  12:     0x5585f083a781 - core::panicking::panic_fmt::hb5178b003b60d015
                               at src/libcore/panicking.rs:85
  13:     0x5585f083a6cd - core::panicking::panic::h732a9e95c599771d
                               at src/libcore/panicking.rs:52
  14:     0x5585f06226e5 - neo::agents::network_agents::bitmex_agent::BitmexAgent::handle_instrument_message::{{closure}}::update_listeners::h7b090aebabb5a3fc
  15:     0x5585f0607bb8 - neo::agents::network_agents::bitmex_agent::BitmexAgent::run_message_loop::{{closure}}::hcc6b07e69052e9c2
  16:     0x5585f05f63a9 - <std::future::GenFuture<T> as core::future::future::Future>::poll::hf96f78cdbf7d85fe
  17:     0x5585f06208ab - tokio::runtime::basic_scheduler::BasicScheduler<P>::block_on::h0400720fcecf70ea
  18:     0x5585f062fbb8 - tokio::runtime::context::enter::hddce36e56d7f4024
  19:     0x5585f05da55c - std::sys_common::backtrace::__rust_begin_short_backtrace::ha82dc02f109d0613
  20:     0x5585f056e364 - std::panicking::try::do_call::h587f0ff2565c3b16
  21:     0x5585f0821407 - __rust_maybe_catch_panic
                               at src/libpanic_unwind/lib.rs:86
  22:     0x5585f0616d73 - core::ops::function::FnOnce::call_once{{vtable.shim}}::h7ec8d306f318ee0f
  23:     0x5585f080844f - <alloc::boxed::Box<F> as core::ops::function::FnOnce<A>>::call_once::h3e0d532261c49537
                               at /rustc/58b834344fc7b9185e7a50db1ff24e5eb07dae5e/src/liballoc/boxed.rs:1016
  24:     0x5585f0820520 - <alloc::boxed::Box<F> as core::ops::function::FnOnce<A>>::call_once::h3490909077392334
                               at /rustc/58b834344fc7b9185e7a50db1ff24e5eb07dae5e/src/liballoc/boxed.rs:1016
  25:     0x5585f0820520 - std::sys_common::thread::start_thread::h80dc27e723d44644
                               at src/libstd/sys_common/thread.rs:13
  26:     0x5585f0820520 - std::sys::unix::thread::Thread::new::thread_start::h5411d298fefe671a
                               at src/libstd/sys/unix/thread.rs:80
  27:     0x7fc570352422 - start_thread
  28:     0x7fc570265bf3 - __GI___clone
  29:                0x0 - <unknown>





*/
