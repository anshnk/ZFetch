mod ascii;
mod system;
mod ui;
mod config;

use ascii::{get_ascii_logo, process_logo_colors};
use system::get_system_info;
use ui::display_output;
use config::Config;
use std::time::Instant;

#[tokio::main]
async fn main() {
    // let start = Instant::now();
    let config = Config::from_exe_dir().unwrap_or_default();
    let info = get_system_info(&config).await;
    let logo = get_ascii_logo(&info.distro_id).await;
    let colored_logo = process_logo_colors(&logo, &config);
    display_output(colored_logo, &info, &config);
    //let elapsed = start.elapsed();
    //println!("\nExecution time: {:.2?}", elapsed); debug
}

// hi from the future