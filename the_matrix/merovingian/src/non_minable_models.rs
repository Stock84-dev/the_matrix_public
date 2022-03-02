use bevy::prelude::*;

#[repr(C)]
#[derive(Clone, Debug, Default, Readable, Writable)]
pub struct Fees {
    pub maker: f32,
    pub taker: f32,
    pub funding: f32,
    pub funding_period: u32,
}

bitflags! {
    #[derive(Default, Serialize, Deserialize, Reflect)]
    pub struct CLFlags: u8 {
        const DEBUG = 1 << 0;
        const TEST = 1 << 1;
        const PROFILE = 1 << 2;
        const FUN_OPTIMIZATIONS = 1 << 3;
        const LOG = 1 << 4;
        const NO_OPTIMIZATIONS = 1 << 5;
        const ASSERTIONS = 1 << 6;
        const APPROX = 1 << 7;
    }
}

bitflags! {
    #[derive(Default, Serialize, Deserialize, Reflect)]
    pub struct ConstructFlags: u8 {
        const PRE_FILTER = 1 << 0;
        const FILTER = 1 << 1;
        const TEST = 1 << 2;
    }
}

bitflags! {
    #[derive(Default, Serialize, Deserialize, Reflect)]
    pub struct BacktestPlotFlags: u8 {
        const TIMESTAMP = 1 << 0;
        const TRADE_CURVE = 1 << 1;
        const TRADE_WINS = 1 << 2;
        const TRADE_LOSSES = 1 << 3;
        const WINS_AND_LOSSES_STREAK = 1 << 4;
        const REL_WINS_AND_LOSSES_STREAK = 1 << 5;
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum ExitCode {
    // 1 is returned when Err(_) is returned
    // 2 is returned when wrong arguments are passed by clap
    // 101 is returned when panic happens
    Success = 3,
    Reload,
    FailedSafely,
    Fatal,
}

#[derive(Clone, Debug, Readable, Writable)]
pub struct MatrixId {
    pub exchange_id: String,
    pub exchange_api_key: String,
}
