use colored::*;
use std::io::BufRead;
use std::io::BufReader;
use tracing::error;
use tracing::info;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
pub const GIT_HASH: &str = env!("GIT_HASH");
pub fn timestamp() -> u32 {
    chrono::offset::Utc::now().timestamp() as u32
}
pub fn build(cargo_pkg_name: &str, cargo_pkg_version: &str, cargo_pkg_repository: &str) -> String {
    format!(
        "\
{} = {{ version = \"{}\" }}
{}/tree/{}",
        cargo_pkg_name.yellow(),
        cargo_pkg_version.magenta(),
        cargo_pkg_repository.yellow(),
        GIT_HASH.magenta()
    )
}
pub fn io_reload_filter(reload_handle: reload::Handle<EnvFilter, Registry>) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(std::io::stdin());
        let mut line = String::new();
        loop {
            _ = reader.read_line(&mut line);
            let filter = EnvFilter::new(line.trim());
            info!(?filter, "Reload");
            if let Err(e) = reload_handle.modify(|x| *x = filter) {
                error!(?e)
            }
            line.clear();
        }
    });
}
