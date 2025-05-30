use crate::system::SystemInfo;
use crate::config::Config;
use crossterm::style::{Color, SetForegroundColor, ResetColor};
use std::io::{self, Write};

fn pad_box_title(title: &str, box_width: usize) -> String {
    let left_pad = "  ";
    let content_width = box_width.saturating_sub(2 + left_pad.len());
    let centered = format!("{:^width$}", title, width = content_width);
    let pad = content_width.saturating_sub(centered.len());
    format!("│{}{}{}│", left_pad, centered, " ".repeat(pad))
}

fn pad_box_line(label: &str, value: &str, box_width: usize) -> String {
    let left_pad = "  ";
    let label_width = 10;
    let content = format!("{:label_width$}: {}", label, value, label_width = label_width);
    let content_width = box_width.saturating_sub(2 + left_pad.len());
    let pad = content_width.saturating_sub(content.len());
    format!("│{}{}{}│", left_pad, content, " ".repeat(pad))
}

pub fn display_output(logo: String, info: &SystemInfo, config: &Config) {
    let logo_lines: Vec<&str> = logo.lines().collect();

    // get info ready as label/value pairs
    let used_mem = parse_gb(info.used_memory.as_deref().unwrap_or("0"));
    let total_mem = parse_gb(info.total_memory.as_deref().unwrap_or("0"));
    let mem_percent = if total_mem > 0.0 {
        (used_mem / total_mem * 100.0).round()
    } else {
        0.0
    };
    let mem_val = format!(
        "{} / {} ({:.0}%)",
        info.used_memory.as_deref().unwrap_or("N/A"),
        info.total_memory.as_deref().unwrap_or("N/A"),
        mem_percent
    );

    let used_swap = parse_gb(info.used_swap.as_deref().unwrap_or("0"));
    let total_swap = parse_gb(info.total_swap.as_deref().unwrap_or("0"));
    let swap_percent = if total_swap > 0.0 {
        (used_swap / total_swap * 100.0).round()
    } else {
        0.0
    };
    let swap_val = format!(
        "{} / {} ({:.0}%)",
        info.used_swap.as_deref().unwrap_or("N/A"),
        info.total_swap.as_deref().unwrap_or("N/A"),
        swap_percent
    );

    // ok so like, only add stuff if the config says so lol
    let mut info_pairs = Vec::new();
    if config.show_distro.unwrap_or(true) {
        info_pairs.push(("Distro".to_string(), info.distro.clone()));
    }
    if config.show_distro_id.unwrap_or(true) {
        info_pairs.push(("Distro ID".to_string(), info.distro_id.clone()));
    }
    if config.show_kernel.unwrap_or(true) {
        info_pairs.push(("Kernel".to_string(), info.kernel.clone()));
    }
    if config.show_cpu.unwrap_or(true) {
        info_pairs.push((
            "CPU".to_string(),
            info.cpu.as_deref().unwrap_or("N/A").to_string(),
        ));
    }
    if config.show_gpu.unwrap_or(true) {
        info_pairs.push((
            "GPU".to_string(),
            info.gpu.as_deref().unwrap_or("N/A").to_string(),
        ));
    }
    if config.show_memory.unwrap_or(true) {
        info_pairs.push(("Memory".to_string(), mem_val.clone()));
    }
    if config.show_swap.unwrap_or(true) {
        info_pairs.push(("Swap".to_string(), swap_val.clone()));
    }
    if config.show_local_ip.unwrap_or(true) {
        info_pairs.push((
            "Local IP".to_string(),
            info.local_ip.as_deref().unwrap_or("N/A").to_string(),
        ));
    }
    if config.show_battery.unwrap_or(true) {
        info_pairs.push((
            "Battery".to_string(),
            info.battery.as_deref().unwrap_or("N/A").to_string(),
        ));
    }
    if config.show_uptime.unwrap_or(true) {
        info_pairs.push((
            "Uptime".to_string(),
            info.uptime.as_deref().unwrap_or("N/A").to_string(),
        ));
    }

    // FIX: store disk label/value as String to avoid reference issues
    for disk in &info.storage {
        let label = format!("Disk ({})", disk.name);
        let ro = if disk.readonly { " [Read-only]" } else { "" };
        let value = format!(
            "{} / {} ({}%) - {}{}",
            disk.used, disk.total, disk.percent, disk.fs_type, ro
        );
        info_pairs.push((label, value));
    }

    // calculate the max content width needed
    let left_pad = "  ";
    let label_width = 10;
    let mut max_content = "System Information".len();
    for (label, value) in &info_pairs {
        let content = format!("{:label_width$}: {}", label, value, label_width = label_width);
        if content.len() > max_content {
            max_content = content.len();
        }
    }
    let box_width = max_content + left_pad.len() + 2; // +2 for borders

    let mut info_lines = vec![
        format!("┌{:─<width$}┐", "", width = box_width - 2),
        pad_box_title("System Information", box_width),
        format!("├{:─<width$}┤", "", width = box_width - 2),
    ];
    for (label, value) in &info_pairs {
        info_lines.push(pad_box_line(label, value, box_width));
    }
    info_lines.push(format!("└{:─<width$}┘", "", width = box_width - 2));

    let logo_width = logo_lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let info_width = box_width;
    let total_width = logo_width + 4 + info_width;
    let term_width = 80;
    let pad_left = if term_width > total_width {
        (term_width - total_width) / 2
    } else {
        0
    };
    let total_lines = logo_lines.len().max(info_lines.len());

    // get colors from config
    let logo_color = config.logo_color.as_deref().and_then(parse_hex_color).unwrap_or(Color::White);
    let info_color = config.color.as_deref().and_then(parse_hex_color).unwrap_or(Color::White);

    for i in 0..total_lines {
        let logo_part = logo_lines.get(i).map_or("", |v| *v);
        let info_part = info_lines.get(i).map_or("", |s| s.as_str());

        // print logo in logo_color, info in info_color
        print!("{space:>pad$}", space = "", pad = pad_left);

        // Logo
        print!("{}", SetForegroundColor(logo_color));
        print!("{:<logo_width$}", logo_part, logo_width = logo_width);

        // separator and info
        print!("{}", SetForegroundColor(info_color));
        if !info_part.is_empty() {
            print!("    {}", info_part);
        }
        println!("{}", ResetColor);
    }

    io::stdout().flush().unwrap();
}

// function to parse "12.34 GB" -> 12.34
fn parse_gb(s: &str) -> f64 {
    s.split_whitespace()
        .next()
        .and_then(|num| num.parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let Ok(rgb) = u32::from_str_radix(hex, 16) {
            let r = ((rgb >> 16) & 0xFF) as u8;
            let g = ((rgb >> 8) & 0xFF) as u8;
            let b = (rgb & 0xFF) as u8;
            return Some(Color::Rgb { r, g, b });
        }
    }
    None
}