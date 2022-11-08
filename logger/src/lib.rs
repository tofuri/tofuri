use chrono::Local;
use colored::*;
use env_logger::Builder;
use log::{Level, LevelFilter};
use std::io::Write;
pub fn init(log_path: bool) {
    fn colored_level(level: Level) -> ColoredString {
        match level {
            Level::Error => level.to_string().red(),
            Level::Warn => level.to_string().yellow(),
            Level::Info => level.to_string().green(),
            Level::Debug => level.to_string().blue(),
            Level::Trace => level.to_string().magenta(),
        }
    }
    let mut builder = Builder::new();
    if log_path {
        builder.format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}{}{}] {}",
                Local::now().format("%H:%M:%S"),
                colored_level(record.level()),
                record.file_static().unwrap().black(),
                ":".black(),
                record.line().unwrap().to_string().black(),
                record.args()
            )
        });
    } else {
        builder.format(|buf, record| writeln!(buf, "[{} {}] {}", Local::now().format("%H:%M:%S"), colored_level(record.level()), record.args()));
    }
    builder.filter(None, LevelFilter::Info).init();
}
