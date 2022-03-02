use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use futures::FutureExt;
use merovingian::non_minable_models::ExitCode;
use mouse::error::Result;
use mouse::log::*;
use mouse::time::{IntoDateTime, Timestamp};
use nebuchadnezzar_core::Exchange;
use thiserror::Error;
use tokio::select;
use tokio::sync::Mutex;

use crate::agents::network_agent::exchange_state::load_last_execution_time;
use crate::agents::network_agent::NetworkClient;
use crate::agents::network_agents::NetworkAgent;
use crate::error::MatrixError;

#[async_trait]
pub trait Ws: Unpin + Send {
    type Message: Send;

    async fn next(&mut self) -> Self::Message;
    async fn close(&mut self) -> Result<()>;
}

async fn run_message_loop_may_panic_or_fail<T>(agent: Arc<Mutex<T>>) -> Result<()>
where
    T: NetworkAgent,
{
    // TODO: can throw error if msg is not parsed properly or is not in text format, handle this
    // with newer version of lib
    let mut ws = process_backlog(&agent).await?;
    info!("Message loop started.");
    let mut ups = 0.;
    let min_timeframe = agent.lock().await.state_mut().min_timeframe() as i64 * 1_000_000_000;
    let now = Utc::now().timestamp_nanos();
    let mut ups_time = now - now % 60_000_000_000 + 60_000_000_000;
    let mut tick_time = now - now % min_timeframe + min_timeframe;
    let mut result: Result<()>;
    loop {
        result = try {
            ups += 1.;
            let now = Utc::now().timestamp_nanos();
            if now >= ups_time {
                ups_time = now - now % 60_000_000_000 + 60_000_000_000;
                trace!("UPS: {}", ups / 60.);
                ups = 0.;
            }
            if now >= tick_time {
                // These 3 ticks in loop are here to ensure that candles_builder will be updated
                // every min_timeframe so that it could add new candle and call models.
                agent
                    .lock()
                    .await
                    .state_mut()
                    .tick_candles_on_all_markets((now / 1_000_000_000) as u32 + 1)
                    .await?;
            }
            let _agent2 = agent.clone();
            tick_time = now - now % min_timeframe + min_timeframe;
            let sleep_time = tick_time - now;
            // Using this timer instead of tokio one because of timer resolution.
            // Tokio timer fires after 1-2 ms where this one after 0.2 ms.
            let mut interval = async_timer::Interval::platform_new(
                core::time::Duration::from_nanos(sleep_time as u64),
            );
            let sleep_fut = interval.as_mut().fuse();
            let ws_fut = Ws::next(&mut ws).fuse();

            // Runs 2 futures, when one completes the other is killed.
            // When future completes a callback is called.
            select! {
                msg = ws_fut => {
                    let mut agent = agent.lock().await;
                    #[cfg(not(feature = "test"))]
                    agent.state_mut().check_for_zion_message().await?;
                    agent.handle_message(msg).await?;
                    let now = Utc::now().timestamp_nanos();
                    if now >= tick_time {
                        agent
                            .state_mut()
                            .tick_candles_on_all_markets((now / 1_000_000_000) as u32 + 1).await?;
                        // Prevent being called again at the start of next loop.
                        tick_time = i64::MAX;
                    }
                },
                _ = sleep_fut => {
                    agent
                        .lock()
                        .await
                        .state_mut()
                        .tick_candles_on_all_markets(Utc::now().timestamp_s() + 1).await?;
                    // Prevent being called again at the start of next loop.
                    tick_time = i64::MAX;
                },
            };
        };
        if result.is_err() {
            break;
        }
    }
    agent.lock().await.state_mut().ws = Some(ws);
    result
}

pub async fn run_message_loop<T>(agent: Arc<Mutex<T>>) -> ExitCode
where
    T: NetworkAgent,
{
    loop {
        if let Err(e) = run_message_loop_may_panic_or_fail(agent.clone()).await {
            match e.downcast_ref::<MatrixError>() {
                None => {
                    // TODO: match on err and return success if err is maintenance
                    error!("{:?}", e);
                    trace!("Killing network agent!");
                    // TODO: if killing wasn't successful send email, sound alarm, display
                    //  notification
                    let mut agent = agent.lock().await;
                    let state = agent.state_mut();
                    match state.kill().await {
                        Ok(_) => {
                            if let Err(e) = state.on_shutdown().await {
                                error!("Shutdown failed: {:#?}", e)
                            }
                            info!("Agent killed successfully!");
                            return ExitCode::FailedSafely;
                        }
                        Err(e) => {
                            error!("FATAL: Killing agent FAILED. {:?}.", e);
                            if let Err(e) = state.on_shutdown().await {
                                error!("Shutdown failed: {:#?}", e);
                            }
                            return ExitCode::Fatal;
                        }
                    }
                }
                Some(MatrixError::WebsocketExhausted) => {
                    let mut agent = agent.lock().await;
                    let result: Result<_> = try {
                        agent.state_mut().reconnect().await?;
                        agent.new_subscribed_web_socket().await?
                    };
                    match result {
                        Ok(ws) => {
                            agent.state_mut().ws = Some(ws);
                        }
                        Err(e) => {
                            error!("Reconnect failed: {:#?}", e);
                            return ExitCode::Fatal;
                        }
                    }
                    continue;
                }
                Some(MatrixError::Shutdown) => return ExitCode::Success,
                Some(MatrixError::Reload) => return ExitCode::Reload,
            }
        }
    }
}

async fn process_backlog<T>(agent: &Arc<Mutex<T>>) -> Result<<T as NetworkAgent>::Websocket>
where
    T: NetworkAgent,
{
    let mut ws = {
        let mut agent = agent.lock().await;
        info!("Catching up...");
        let mut last_execution_time =
            load_last_execution_time(agent.state_mut().client().exchange().name()).await?;
        if last_execution_time != u64::MAX {
            // To not fetch already executed orders
            last_execution_time += 1_000_000_000;
        }
        let executions = agent.catch_up(last_execution_time.into_date_time()).await?;
        agent.state_mut().catch_up(executions).await?;

        info!("Processing websocket backlog...");
        agent.state_mut().ws.take().unwrap()
    };
    loop {
        // Using this timer instead of tokio one because of timer resolution.
        // Tokio timer fires after 1-2 ms where this one after 0.2 ms.
        let sleep_fut = tokio::time::sleep(tokio::time::Duration::from_millis(1)).fuse();
        let ws_fut = Ws::next(&mut ws).fuse();

        // Runs 2 futures, when one completes the other is killed.
        // When future completes a callback is called.
        // If there is backlog in sink the 'next' method of a stream should resolve immediately
        let result: Result<()> = select! {
            msg = ws_fut => {
                agent.lock().await.handle_message(msg).await
            },
            _ = sleep_fut => {
                Err(MatrixError::Shutdown.into())
            },
        };
        if let Err(e) = result {
            match e.downcast::<MatrixError>() {
                Err(e) => return Err(e),
                Ok(MatrixError::Shutdown) => break,
                Ok(e) => return Err(e.into()),
            }
        }
    }

    Ok(ws)
}
