// use log::max_level;

use crate::{
    console::PLATFORM,
    // gpio::{GPIO_BASE, init_gpio_as_output, set_gpio_output},
};
use core::str::FromStr;
// use log::{Level, LevelFilter};

#[macro_export]
#[allow(unused)]
macro_rules! println {
    () => {
        $crate::print!("\n\r")
    };
    ($($arg:tt)*) => {{
        use core::fmt::Write;
            #[allow(static_mut_refs)]
            let console = unsafe { PLATFORM.console.as_mut().unwrap()};
            console.write_fmt(core::format_args!($($arg)*)).unwrap();
            console.write_str("\n\r").unwrap();
    }};
}

// /// Simple logger implementation for RustSBI that supports colored output.
// pub struct Logger;

// impl Logger {
//     /// Initialize the logger with log level from RUST_LOG env var or default to Info.
//     pub fn init() -> Result<(), log::SetLoggerError> {
//         // Set max log level from LOG_LEVEL from config file, otherwise use Info
//         let max_level = LevelFilter::from_str("INFO").unwrap_or(LevelFilter::Info);

//         log::set_max_level(max_level);

//         println!("这里还有啥问题么？");
//         unsafe { log::set_logger_racy(&Logger) }
//     }
// }

// impl log::Log for Logger {
//     // Always enable logging for all log levels
//     #[inline]
//     fn enabled(&self, _metadata: &log::Metadata) -> bool {
//         true
//     }

//     // Log messages with color-coded levels
//     #[inline]
//     fn log(&self, record: &log::Record) {
//         // ANSI color codes for different log levels
//         const ERROR_COLOR: u8 = 31; // Red
//         const WARN_COLOR: u8 = 93; // Bright yellow
//         const INFO_COLOR: u8 = 32; // Green
//         const DEBUG_COLOR: u8 = 36; // Cyan
//         const TRACE_COLOR: u8 = 90; // Bright black

//         let color_code = match record.level() {
//             Level::Error => ERROR_COLOR,
//             Level::Warn => WARN_COLOR,
//             Level::Info => INFO_COLOR,
//             Level::Debug => DEBUG_COLOR,
//             Level::Trace => TRACE_COLOR,
//         };

//         println!(
//             "\x1b[1;37m[RustSBI] \x1b[1;{color_code}m{:^5}\x1b[0m - {}",
//             record.level(),
//             record.args(),
//         );
//     }

//     // No-op flush since we use println! which is already line-buffered
//     #[inline]
//     fn flush(&self) {}
// }
