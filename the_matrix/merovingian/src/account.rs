use std::f32::NAN;
use std::fmt::{Display, Formatter};

use mouse::field_names;
use num_enum::{IntoPrimitive, TryFromPrimitive};

field_names! {
    #[repr(C)]
    #[derive(Debug, Clone, Readable, Writable, Serialize)]
    pub struct Account {
        pub bought_id: u32,
        pub entry_price: f32,
        pub max_balance: f32,
        // NOTE: This is not in %.
        pub max_drawdown: f32,
        pub balance: f32,
        pub position: f32,
        pub n_trades: f32,
        pub taker_fee: f32,
    }
}

impl Account {
    pub fn meets_requirements(&self) -> bool {
        return self.bought_id != 1;
    }
}

impl Default for Account {
    fn default() -> Self {
        Account {
            bought_id: 0,
            entry_price: 0.0,
            max_balance: 0.0,
            max_drawdown: 0.0,
            balance: 1.0,
            position: 0.0,
            n_trades: 0.0,
            taker_fee: 0.0,
        }
    }
}

field_names! {
    #[repr(C)]
    #[derive(PartialEq, Debug, Clone, Readable, Writable, Serialize)]
    pub struct StatAccount {
        pub can_record: f32,
        pub bought_id: f32,
        pub entry_price: f32,
        pub max_balance: f32,
        // NOTE: This is not in %.
        pub max_drawdown: f32,
        pub balance: f32,
        pub position: f32,
        pub n_trades: f32,
        pub taker_fee: f32,

        pub previous_balance: f32,
        pub avg_bars_in_win_trades: f32,
        pub avg_bars_in_loss_trades: f32,
        pub avg_bars_in_trades: f32,
        pub avg_p_time_in_trades: f32,
        pub win_rate_p: f32,
        pub n_win_trades: f32,
        pub n_loss_trades: f32,
        pub buy_and_hold_return: f32,
        pub avg_risk_p: f32,
        pub avg_reward_p: f32,
        pub risk_to_reward_ratio: f32,
        pub expectancy_r: f32,
        pub expected_return_1y_p: f32,
        pub expected_return_1m_p: f32,
        pub expected_return_1d_p: f32,
        pub max_p_gain: f32,
        pub max_p_loss: f32,
        pub max_p_streak_win: f32,
        pub p_streak: f32,
        pub max_p_streak_loss: f32,
        pub max_n_streak_win: f32,
        pub n_streak: f32,
        pub max_n_streak_loss: f32,
        pub maker_fee: f32,
        pub funding_fee: f32,
        pub volatility_p: f32,
        pub sharpe_ratio: f32,
        // NOTE: All fields must be 4 bytes in size.
    }
}

impl StatAccount {
    pub fn field_names_black_list() -> &'static [&'static str] {
        &[
            "entry_price",
            "can_record",
            "bought_id",
            "position",
            "taker_fee",
            "maker_fee",
            "funding_fee",
            "buy_and_hold_return",
            "previous_balance",
            "p_streak",
            "n_streak",
        ]
    }

    pub fn field_names_to_plot() -> impl Iterator<Item = &'static &'static str> {
        fn f(x: &&&'static str) -> bool {
            !StatAccount::field_names_black_list().contains(x)
        }
        StatAccount::NAMES
            .iter()
            .filter(f as fn(&&&'static str) -> bool)
    }

    pub fn meets_requirements(&self) -> bool {
        self.bought_id != 1.
    }

    pub fn expectancy_p(&self) -> f32 {
        self.expectancy_r * self.avg_risk_p
    }

    pub fn statistical_edge(&self) -> f32 {
        self.win_rate_p - (100. - self.win_rate_p)
    }
}

impl Default for StatAccount {
    fn default() -> Self {
        Self {
            can_record: NAN,
            bought_id: NAN,
            entry_price: NAN,
            max_balance: NAN,
            max_drawdown: NAN,
            balance: NAN,
            position: NAN,
            n_trades: NAN,
            taker_fee: NAN,
            previous_balance: NAN,
            avg_bars_in_win_trades: NAN,
            avg_bars_in_loss_trades: NAN,
            avg_bars_in_trades: NAN,
            avg_p_time_in_trades: NAN,
            win_rate_p: NAN,
            n_win_trades: NAN,
            n_loss_trades: NAN,
            buy_and_hold_return: NAN,
            avg_risk_p: NAN,
            avg_reward_p: NAN,
            risk_to_reward_ratio: NAN,
            expectancy_r: NAN,
            expected_return_1y_p: NAN,
            expected_return_1m_p: NAN,
            expected_return_1d_p: NAN,
            max_p_gain: NAN,
            max_p_loss: NAN,
            max_p_streak_win: NAN,
            p_streak: NAN,
            max_p_streak_loss: NAN,
            max_n_streak_win: NAN,
            n_streak: NAN,
            max_n_streak_loss: NAN,
            maker_fee: NAN,
            funding_fee: NAN,
            volatility_p: NAN,
            sharpe_ratio: NAN,
        }
    }
}

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(usize)]
pub enum StatTypes {
    EquityCurve = 0,
    TradeCurve,
    TradeWins,
    TradeLosses,
    WinsAndLossesStreak,
    PWinsAndLossesStreak,
    Price,
}

impl Display for StatTypes {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            StatTypes::EquityCurve => write!(f, "Equity Curve"),
            StatTypes::TradeCurve => write!(f, "Trade Curve"),
            StatTypes::TradeWins => write!(f, "Trade Wins"),
            StatTypes::TradeLosses => write!(f, "Trade Losses"),
            StatTypes::WinsAndLossesStreak => write!(f, "Wins And Losses Streak"),
            StatTypes::PWinsAndLossesStreak => write!(f, "Percent Wins And Losses Streak"),
            StatTypes::Price => write!(f, "Price"),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Readable, Writable)]
pub struct TradeAccount {
    pub entry_price: f32,
    pub balance: f32,
    pub position: f32,
    pub risk_activated: f32,
}

impl Default for TradeAccount {
    fn default() -> Self {
        TradeAccount {
            entry_price: NAN,
            balance: NAN,
            position: NAN,
            risk_activated: 0.0,
        }
    }
}
