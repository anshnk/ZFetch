mod ascii;
mod config;
mod system;
mod ui;

use ascii::{get_ascii_logo, process_logo_colors};
use config::Config;
use system::get_system_info;
use ui::display_output;
// use std::time::Instant;

#[tokio::main]
async fn main() {
    // let start = Instant::now();
    let config = Config::from_exe_dir().unwrap_or_default();
    let info = get_system_info(&config).await;
    let logo = get_ascii_logo(&info.distro_id);
    let colored_logo = process_logo_colors(&logo, &config);
    display_output(colored_logo, &info, &config);
    // let elapsed = start.elapsed();
    // println!("\nExecution time: {:.2?}", elapsed); //uncomment everything for debugging speeds
}

// hi from the future
