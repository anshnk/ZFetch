use crate::config::Config;
use crate::system::SystemInfo;
use crate::terminal::colors::ColorSystem;
use regex::Regex;
use std::io::{self, Write};
use std::sync::LazyLock;

static ANSI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[0-9;]*m").unwrap());

fn visible_width(s: &str) -> usize {
    ANSI_RE.replace_all(s, "").len()
}

fn border_segment(colors: &ColorSystem, ch: char) -> String {
    if colors.enabled() {
        let color = colors.box_color();
        if !color.is_empty() {
            return format!("{color}{ch}{}", colors.reset());
        }
    }
    ch.to_string()
}

fn color_border_line(line: String, colors: &ColorSystem) -> String {
    if colors.enabled() {
        let color = colors.box_color();
        if !color.is_empty() {
            return format!("{color}{line}{}", colors.reset());
        }
    }
    line
}

fn pad_box_title(title: &str, box_width: usize, colors: &ColorSystem) -> String {
    let left_pad = "  ";
    let content_width = box_width.saturating_sub(2 + left_pad.len());
    let centered = format!("{:^width$}", title, width = content_width);
    let colored = if colors.enabled() && !title.trim().is_empty() {
        let color = colors.title_color();
        if color.is_empty() {
            centered.clone()
        } else {
            format!("{color}{centered}{}", colors.reset())
        }
    } else {
        centered
    };
    let left_border = border_segment(colors, '│');
    let right_border = border_segment(colors, '│');

    let mut line = String::with_capacity(
        left_border.len() + left_pad.len() + colored.len() + right_border.len(),
    );
    line.push_str(&left_border);
    line.push_str(left_pad);
    line.push_str(&colored);
    line.push_str(&right_border);
    line
}

fn pad_box_line(label: &str, value: &str, box_width: usize, colors: &ColorSystem) -> String {
    let left_pad = "  ";
    let label_width = 10;
    let label_with_colon = format!("{:label_width$}: ", label, label_width = label_width);

    let colored_label = if colors.enabled() && !label.trim().is_empty() {
        let color = colors.key_color();
        if color.is_empty() {
            label_with_colon.clone()
        } else {
            format!("{color}{label_with_colon}{}", colors.reset())
        }
    } else {
        label_with_colon.clone()
    };

    let colored_value = if colors.enabled() && !value.is_empty() {
        let color = colors.value_color();
        if color.is_empty() {
            value.to_string()
        } else {
            format!("{color}{value}{}", colors.reset())
        }
    } else {
        value.to_string()
    };

    let content_width = box_width.saturating_sub(2 + left_pad.len());
    let plain_len = label_with_colon.len() + value.len();
    let pad = content_width.saturating_sub(plain_len);
    let padding = " ".repeat(pad);

    let left_border = border_segment(colors, '│');
    let right_border = border_segment(colors, '│');

    let mut line = String::with_capacity(
        left_border.len()
            + left_pad.len()
            + colored_label.len()
            + colored_value.len()
            + padding.len()
            + right_border.len(),
    );
    line.push_str(&left_border);
    line.push_str(left_pad);
    line.push_str(&colored_label);
    line.push_str(&colored_value);
    line.push_str(&padding);
    line.push_str(&right_border);
    line
}

pub fn display_output(logo: String, info: &SystemInfo, config: &Config, colors: &ColorSystem) {
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

    if config.show_storage.unwrap_or(true) {
        for disk in &info.storage {
            let label = format!("Disk ({})", disk.name);
            let ro = if disk.readonly { " [Read-only]" } else { "" };
            let value = format!(
                "{} / {} ({}%) - {}{}",
                disk.used, disk.total, disk.percent, disk.fs_type, ro
            );
            info_pairs.push((label, value));
        }
    }

    let user_host = if config.show_user_host.unwrap_or(true) {
        match (&info.username, &info.hostname) {
            (Some(user), Some(host)) => format!("{}@{}", user, host),
            (Some(user), None) => user.clone(),
            (None, Some(host)) => host.clone(),
            (None, None) => String::new(),
        }
    } else {
        String::new()
    };
    let title = if !user_host.is_empty() {
        format!("{} | System Information", user_host)
    } else {
        "System Information".to_string()
    };

    let left_pad = "  ";
    let label_width = 10;

    let mut max_content = "System Information".len();
    for (label, value) in &info_pairs {
        for line in value.lines() {
            let content = format!(
                "{:label_width$}: {}",
                label,
                line,
                label_width = label_width
            );
            if content.len() > max_content {
                max_content = content.len();
            }
        }
    }
    let box_width = max_content + left_pad.len() + 2;

    let mut info_lines = vec![
        color_border_line(format!("┌{:─<width$}┐", "", width = box_width - 2), colors),
        pad_box_title(&title, box_width, colors),
        color_border_line(format!("├{:─<width$}┤", "", width = box_width - 2), colors),
    ];
    for (label, value) in &info_pairs {
        for (i, line) in value.lines().enumerate() {
            let label_str = if i == 0 { label.as_str() } else { "" };
            info_lines.push(pad_box_line(label_str, line, box_width, colors));
        }
    }
    info_lines.push(color_border_line(
        format!("└{:─<width$}┘", "", width = box_width - 2),
        colors,
    ));

    let logo_lines = logo.lines().collect::<Vec<_>>();
    let logo_width = logo_lines
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0);
    let info_width = box_width;
    let horizontal_gap = 4;
    let gap_str = " ".repeat(horizontal_gap);
    let total_width = logo_width + horizontal_gap + info_width;
    let term_width = 80;
    let pad_left = if term_width > total_width {
        (term_width - total_width) / 2
    } else {
        0
    };
    let total_lines = logo_lines.len().max(info_lines.len());

    let outer_padding = 1;
    for _ in 0..outer_padding {
        print!("{space:>pad$}", space = "", pad = pad_left);
        if logo_width > 0 {
            print!("{:width$}", "", width = logo_width);
        }
        print!("{}", gap_str);
        println!("{:width$}", "", width = info_width);
    }

    for i in 0..total_lines {
        let logo_part = logo_lines.get(i).copied().unwrap_or("");
        let info_part = info_lines.get(i).map(|s| s.as_str()).unwrap_or("");

        print!("{space:>pad$}", space = "", pad = pad_left);

        print!("{}", logo_part);
        let logo_visible = visible_width(logo_part);
        let pad_amount = logo_width.saturating_sub(logo_visible);
        if pad_amount > 0 {
            print!("{}", " ".repeat(pad_amount));
        }

        if !info_part.is_empty() {
            print!("{}{}", gap_str, info_part);
        } else {
            print!("{}{:width$}", gap_str, "", width = info_width);
        }

        if colors.enabled() {
            print!("{}", colors.reset());
        }

        println!();
    }

    for _ in 0..outer_padding {
        print!("{space:>pad$}", space = "", pad = pad_left);
        if logo_width > 0 {
            print!("{:width$}", "", width = logo_width);
        }
        print!("{}", gap_str);
        println!("{:width$}", "", width = info_width);
    }

    io::stdout().flush().unwrap();
}

fn parse_gb(s: &str) -> f64 {
    s.split_whitespace()
        .next()
        .and_then(|num| num.parse::<f64>().ok())
        .unwrap_or(0.0)
}
