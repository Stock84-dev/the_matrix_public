use std::sync::Once;

use fern::colors::{Color, ColoredLevelConfig};
use fern::Dispatch;

#[macro_export]
macro_rules! a_eq {
    ($left:expr, $right:expr) => {{
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    // The reborrows below are intentional. Without them, the stack slot for the
                    // borrow is initialized even before the values are compared, leading to a
                    // noticeable slow down.
                    panic!(
                        r#"assertion failed:
    left: `({} = {:?})` 
    right: `({} = {:?})`
"#,
                        stringify!($left),
                        &*left_val,
                        stringify!($right),
                        &*right_val
                    );
                }
            }
        }
    }};
}

static INIT_LOGGING: Once = Once::new();

pub fn configure_logging_once() {
    INIT_LOGGING.call_once(|| {
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
        Dispatch::new()
            .chain(
                // terminal
                Dispatch::new()
                    .format(move |out, message, record| {
                        out.finish(format_args!(
                            "{color_line}[{date}][{target}:{line}][{level}{color_line}] \
                             {message}\x1B[0m",
                            color_line = format_args!(
                                "\x1B[{}m",
                                colors_line.get_color(&record.level()).to_fg_str()
                            ),
                            date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"),
                            target = record.target(),
                            line = record.line().unwrap(),
                            level = colors_level.color(record.level()),
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
                    .chain(std::io::stdout()),
            )
            .apply()
            .unwrap();
    });
}

pub use merovingian;
use merovingian::candles::Candles;
use merovingian::non_minable_models::CLFlags;
use ocl::*;
use opencl::setup_program_builder;

// This is only used for unit tests. If it's included in module tests that have cfg(test)
// attribute then it will not be visible in other crates even though it's used during
// testing. I don't know how to compile dependency with test attribute during testing.
// If it's included in external crate it will not compile merovingian because test_helper
// depends on merovingian and merovingian on test_helper.
lazy_static::lazy_static! {
    pub static ref MODELS_PATH_STRING: String = "/home/stock/projects/the_matrix/merovingian/src/kernel_managers/models/".into();

    pub static ref CANDLES: Candles = {
        // logs don't work in tests right now but there is an active issue about it
        configure_logging_once();
        Candles::from_binary_aos("../test_data/bitmex/candles/XBTUSD1d.bin").unwrap()
    };

    pub static ref PROGRAM: Program = {
        setup_program_builder(
            Program::builder(),
            &DEVICE,
            CLFlags::TEST,
            DeviceType::CPU,
            &mut Vec::new(),
            None,
        ).unwrap().build(&CONTEXT).unwrap()
    };

    pub static ref QUEUE: Queue = {
        Queue::new(&CONTEXT, *DEVICE, None).unwrap()
    };

    pub static ref DEVICE: Device = {
        CONTEXT.devices()[0].clone()
    };

    pub static ref CONTEXT: Context = {
        // logs don't work in tests right now but there is an active issue about it
        configure_logging_once();
        for platform in Platform::list() {
            for d in Device::list(platform, Some(DeviceType::CPU)).unwrap() {
                return Context::builder()
                    .platform(platform)
                    .devices(d)
                    .build().unwrap();
            }
        }
        panic!("Could not build context!");
    };
}
