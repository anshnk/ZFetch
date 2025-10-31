mod ascii;
mod common;
mod config;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod system;
mod terminal;
mod ui;
#[cfg(target_os = "windows")]
mod windows;

use ascii::{get_ascii_logo, process_logo_colors};
use config::Config;
#[cfg(not(unix))]
use std::io::IsTerminal;
//use std::time::Instant;
use system::get_system_info;
use terminal::colors::ColorSystem;
use terminal::theme::detect_terminal_theme;
use ui::display_output;

// uncomment lines 11, 21, 42, and 43 for debugging, i'll add a argument for getting that laterrr

#[tokio::main]
async fn main() {
    //let start = Instant::now();
    let config = Config::from_exe_dir().unwrap_or_default();
    let info = get_system_info(&config).await;
    let logo = get_ascii_logo(&info.distro_id);

    let stdout_is_tty = stdout_is_terminal();
    let theme = if stdout_is_tty {
        detect_terminal_theme(true)
    } else {
        None
    };
    let colors = ColorSystem::new(
        &info.distro_id,
        &info.distro,
        &config,
        theme.as_ref(),
        stdout_is_tty,
    );

    let colored_logo = process_logo_colors(&logo, &colors);
    display_output(colored_logo, &info, &config, &colors);
    //let elapsed = start.elapsed();
    //println!("\nExecution time: {:.2?}", elapsed);
}

#[cfg(unix)]
fn stdout_is_terminal() -> bool {
    use std::io::IsTerminal;
    let tty = unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 };
    tty || std::io::stdout().is_terminal()
}

#[cfg(not(unix))]
fn stdout_is_terminal() -> bool {
    std::io::stdout().is_terminal()
}
