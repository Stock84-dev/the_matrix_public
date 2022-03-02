use std::path::Path;
use std::thread;

use fern::colors::{Color, ColoredLevelConfig};
use fern::Dispatch;
pub use log::*;
use serde::{Deserialize, Serialize};

use crate::error::Result;

pub fn conf(config: LogConfig) -> Result<()> {
    if let LevelFilter::Off = config.level {
        return Ok(());
    }

    Ok(())
}

pub fn configure_logging() {
    // configure colors for the whole line
    let colors_line = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::White)
        // depending on the terminals color scheme, this is the same as the background color
        .trace(Color::BrightBlack);

    // configure colors for the name of the level.
    // since almost all of them are the some as the color for the whole line, we
    // just clone `colors_line` and overwrite our changes
    let colors_level = colors_line.clone().info(Color::Green);
    let mut dispatch = fern::Dispatch::new().chain(
        // terminal
        Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{color_line}[{date}][{target}:{line}][{level}{color_line}:{thread_id}] \
                     {message}\x1B[0m",
                    color_line = format_args!(
                        "\x1B[{}m",
                        colors_line.get_color(&record.level()).to_fg_str()
                    ),
                    date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                    target = record.target(),
                    line = record.line().unwrap(),
                    level = colors_level.color(record.level()),
                    thread_id = thread::current().id().as_u64(),
                    message = message,
                ));
            })
            .level(log::LevelFilter::Trace)
            // Set level to Trace in order to see REST API requests and responses.
            .level_for("tokio_tungstenite", log::LevelFilter::Info)
            .level_for("tungstenite", log::LevelFilter::Info)
            .level_for("hyper", log::LevelFilter::Info)
            .level_for("want", log::LevelFilter::Info)
            .level_for("mio", log::LevelFilter::Info)
            .level_for("tracing", log::LevelFilter::Info)
            .level_for("matrix_core", log::LevelFilter::Trace)
            .chain(std::io::stdout()),
    );
    if !cfg!(feature = "test") {
        dispatch = dispatch.chain(
            // file
            Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "{}[{}:{}][{}:{}] {}",
                        chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S.%3f]"),
                        record.target(),
                        record.line().unwrap(),
                        record.level(),
                        thread::current().id().as_u64(),
                        message
                    ))
                })
                .level(log::LevelFilter::Trace)
                .level_for("tokio_tungstenite", log::LevelFilter::Info)
                .level_for("tungstenite", log::LevelFilter::Info)
                .level_for("want", log::LevelFilter::Info)
                .level_for("mio", log::LevelFilter::Info)
                .chain(
                    fern::log_file(format!(
                        "/home/stock/data/Documents/Projects/the_matrix/logs/{}",
                        chrono::Local::now()
                            .format("%Y-%m-%d %H:%M:%S.log")
                            .to_string()
                    ))
                    .unwrap(),
                ),
        )
    }
    dispatch.apply().unwrap();
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LogConfig {
    log_path: String,
    log_opencl: bool,
    log_to: LogTo,
    #[serde(with = "MyLevelFilter")]
    level: LevelFilter,
    levels: Vec<SpecificLevel>,
    #[serde(with = "MyColoredLevelConfig")]
    level_colors: ColoredLevelConfig,
}

impl LogConfig {
    pub fn off() -> LogConfig {
        LogConfig {
            log_path: "".to_string(),
            log_opencl: false,
            log_to: LogTo::Console,
            level: LevelFilter::Off,
            levels: vec![],
            level_colors: default_level_colors(),
        }
    }

    pub fn test() -> LogConfig {
        LogConfig {
            log_path: "".to_string(),
            log_opencl: true,
            log_to: LogTo::Console,
            level: LevelFilter::Trace,
            levels: vec![],
            level_colors: default_level_colors(),
        }
    }

    pub fn configure(&self) -> Result<()> {
        std::panic::set_hook(Box::new(|e| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            error!("{}\n{}", e, backtrace);
        }));
        if let LevelFilter::Off = self.level {
            return Ok(());
        }

        match self.log_to {
            LogTo::Console => self.new_console_dispatch().level(self.level),
            LogTo::File => self.new_file_dispatch()?.level(self.level),
            LogTo::ConsoleAndFile => Dispatch::new()
                .chain(self.new_console_dispatch())
                .chain(self.new_file_dispatch()?)
                .level(self.level),
        }
        .apply()?;

        Ok(())
    }

    fn new_dispatch(&self) -> Dispatch {
        let level_colors = self.level_colors.clone();
        let mut dispatch = Dispatch::new().format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{date}][{target}:{line}][{level}{color_line}:{thread_id}] \
                 {message}\x1B[0m",
                color_line = format_args!(
                    "\x1B[{}m",
                    level_colors.get_color(&record.level()).to_fg_str()
                ),
                date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                target = record.target(),
                line = record.line().unwrap(),
                level = level_colors.color(record.level()),
                thread_id = thread::current().id().as_u64(),
                message = message,
            ));
        });
        for specific_level in &self.levels {
            dispatch = dispatch.level_for(specific_level.module.clone(), specific_level.level);
        }
        dispatch
    }

    fn new_console_dispatch(&self) -> Dispatch {
        self.new_dispatch().chain(std::io::stdout())
    }

    fn new_file_dispatch(&self) -> Result<Dispatch> {
        Ok(self
            .new_dispatch()
            .chain(fern::log_file(Path::new(&self.log_path).join(
                format!("{}-{}.log",
                            chrono::Local::now()
                                .format("%Y-%m-%d %H:%M:%S.log"),
                            std::env::current_exe()?.file_name().unwrap().to_str().unwrap()
                    ),
            ))?))
    }
}

impl Default for LogConfig {
    fn default() -> LogConfig {
        LogConfig::off()
    }
}

fn default_level_colors() -> ColoredLevelConfig {
    ColoredLevelConfig {
        error: Color::Red,
        warn: Color::Yellow,
        info: Color::Green,
        debug: Color::White,
        trace: Color::BrightBlack,
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SpecificLevel {
    module: String,
    #[serde(with = "MyLevelFilter")]
    level: LevelFilter,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum LogTo {
    Console,
    File,
    ConsoleAndFile,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "ColoredLevelConfig")]
struct MyColoredLevelConfig {
    #[serde(with = "MyColor")]
    error: Color,
    #[serde(with = "MyColor")]
    warn: Color,
    #[serde(with = "MyColor")]
    info: Color,
    #[serde(with = "MyColor")]
    debug: Color,
    #[serde(with = "MyColor")]
    trace: Color,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Color")]
pub enum MyColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "LevelFilter")]
pub enum MyLevelFilter {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
