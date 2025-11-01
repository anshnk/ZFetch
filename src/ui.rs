use crate::config::Config;
use crate::system::SystemInfo;
use crate::terminal::colors::ColorSystem;
use regex::Regex;
use std::env;
use std::io::{self, Write};
use std::sync::LazyLock;

static ANSI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[0-9;]*m").unwrap());
const LEFT_PAD: &str = "  ";
const LABEL_WIDTH: usize = 10;
const HORIZONTAL_GAP_DEFAULT: usize = 4;
const RIGHT_PADDING: usize = 2;
const MIN_TERMINAL_WIDTH: usize = 40;

fn visible_width(s: &str) -> usize {
    ANSI_RE.replace_all(s, "").len()
}

fn truncate_visible(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut count = 0;
    for ch in text.chars() {
        if count >= max_width {
            break;
        }
        result.push(ch);
        count += 1;
    }
    result
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
    let content_width = box_width.saturating_sub(2 + LEFT_PAD.len());
    let truncated_title = truncate_visible(title, content_width);
    let padding = content_width.saturating_sub(truncated_title.len());
    let left_pad_count = padding / 2;
    let right_pad_count = padding - left_pad_count;

    let mut centered = String::with_capacity(content_width);
    for _ in 0..left_pad_count {
        centered.push(' ');
    }
    centered.push_str(&truncated_title);
    for _ in 0..right_pad_count {
        centered.push(' ');
    }

    let colored = if colors.enabled() && !truncated_title.trim().is_empty() {
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
        left_border.len() + LEFT_PAD.len() + colored.len() + right_border.len(),
    );
    line.push_str(&left_border);
    line.push_str(LEFT_PAD);
    line.push_str(&colored);
    line.push_str(&right_border);
    line
}

fn pad_box_line(label: &str, value: &str, box_width: usize, colors: &ColorSystem) -> String {
    let content_width = box_width.saturating_sub(2 + LEFT_PAD.len());
    let mut label_area_width = LABEL_WIDTH + 2;
    if label_area_width > content_width {
        label_area_width = content_width;
    }

    let colon_space = if label_area_width >= 2 {
        2
    } else {
        label_area_width
    };
    let effective_label_width = label_area_width.saturating_sub(colon_space);
    let truncated_label = truncate_visible(label, effective_label_width);

    let mut label_with_colon = if effective_label_width > 0 {
        format!(
            "{:label_width$}",
            truncated_label,
            label_width = effective_label_width
        )
    } else {
        String::new()
    };

    if colon_space == 2 {
        label_with_colon.push_str(": ");
    } else if colon_space == 1 {
        label_with_colon.push(':');
    }

    let available_value_width = content_width.saturating_sub(label_with_colon.len());
    let truncated_value = truncate_visible(value, available_value_width);

    let colored_label = if colors.enabled() && !label_with_colon.trim().is_empty() {
        let color = colors.key_color();
        if color.is_empty() {
            label_with_colon.clone()
        } else {
            format!("{color}{label_with_colon}{}", colors.reset())
        }
    } else {
        label_with_colon.clone()
    };

    let colored_value = if colors.enabled() && !truncated_value.is_empty() {
        let color = colors.value_color();
        if color.is_empty() {
            truncated_value.clone()
        } else {
            format!("{color}{truncated_value}{}", colors.reset())
        }
    } else {
        truncated_value.clone()
    };

    let plain_len = label_with_colon.len() + truncated_value.len();
    let pad = content_width.saturating_sub(plain_len);
    let padding = " ".repeat(pad);

    let left_border = border_segment(colors, '│');
    let right_border = border_segment(colors, '│');

    let mut line = String::with_capacity(
        left_border.len()
            + LEFT_PAD.len()
            + colored_label.len()
            + colored_value.len()
            + padding.len()
            + right_border.len(),
    );
    line.push_str(&left_border);
    line.push_str(LEFT_PAD);
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
        let mut cpu_value = info.cpu.as_deref().unwrap_or("N/A").to_string();
        if let Some(freq) = info.cpu_max_frequency.as_deref() {
            if !freq.is_empty() {
                if cpu_value == "N/A" {
                    cpu_value = format!("({})", freq);
                } else {
                    cpu_value.push(' ');
                    cpu_value.push('(');
                    cpu_value.push_str(freq);
                    cpu_value.push(')');
                }
            }
        }
        info_pairs.push(("CPU".to_string(), cpu_value));
    }
    if config.show_gpu.unwrap_or(true) {
        if info.gpus.is_empty() {
            info_pairs.push(("GPU".to_string(), "No GPUs detected.".to_string()));
        } else {
            for (idx, gpu) in info.gpus.iter().enumerate() {
                let label = format!("GPU {}", idx + 1);
                let value = format!("{} [{}]", gpu.name, gpu.kind.label());
                info_pairs.push((label, value));
            }
        }
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

    let logo_lines = logo.lines().collect::<Vec<_>>();
    let logo_width = logo_lines
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0);

    let mut term_width = detect_terminal_width();
    if term_width < MIN_TERMINAL_WIDTH {
        term_width = MIN_TERMINAL_WIDTH;
    }

    let mut horizontal_gap = HORIZONTAL_GAP_DEFAULT;
    if term_width <= logo_width + horizontal_gap {
        horizontal_gap = term_width.saturating_sub(logo_width);
    }

    let min_box_width = LEFT_PAD.len() + 2;
    let mut available_info_width = term_width.saturating_sub(logo_width + horizontal_gap);
    if available_info_width < min_box_width {
        available_info_width = min_box_width;
    }

    let inner_limit = available_info_width.saturating_sub(min_box_width);
    let mut max_line_content = truncate_visible(&title, inner_limit).len();

    for (label, value) in &info_pairs {
        for line in value.lines() {
            let mut label_area_width = LABEL_WIDTH + 2;
            if label_area_width > inner_limit {
                label_area_width = inner_limit;
            }
            let colon_space = if label_area_width >= 2 {
                2
            } else {
                label_area_width
            };
            let effective_label_width = label_area_width.saturating_sub(colon_space);
            let truncated_label = truncate_visible(label, effective_label_width);

            let mut label_with_colon = if effective_label_width > 0 {
                format!(
                    "{:label_width$}",
                    truncated_label,
                    label_width = effective_label_width
                )
            } else {
                String::new()
            };

            if colon_space == 2 {
                label_with_colon.push_str(": ");
            } else if colon_space == 1 {
                label_with_colon.push(':');
            }

            let available_value_width = inner_limit.saturating_sub(label_with_colon.len());
            let truncated_value = truncate_visible(line, available_value_width);
            let line_length = label_with_colon.len() + truncated_value.len();
            if line_length > max_line_content {
                max_line_content = line_length;
            }
        }
    }

    if max_line_content > inner_limit {
        max_line_content = inner_limit;
    }
    let max_content = (max_line_content + RIGHT_PADDING).min(inner_limit);
    let mut box_width = max_content + min_box_width;
    if box_width > available_info_width {
        box_width = available_info_width;
    }
    if box_width < min_box_width {
        box_width = min_box_width;
    }

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

    let info_width = box_width;
    let gap_str = " ".repeat(horizontal_gap);
    let total_width = logo_width + horizontal_gap + info_width;
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

fn detect_terminal_width() -> usize {
    if let Ok(columns) = env::var("COLUMNS") {
        if let Ok(val) = columns.trim().parse::<usize>() {
            if val >= MIN_TERMINAL_WIDTH {
                return val;
            }
        }
    }

    #[cfg(unix)]
    {
        use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};
        unsafe {
            let mut ws: winsize = std::mem::zeroed();
            if ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
                return ws.ws_col as usize;
            }
        }
    }

    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Console::{
            GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO, STD_OUTPUT_HANDLE,
        };

        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if handle != HANDLE(0) {
                let mut info = MaybeUninit::<CONSOLE_SCREEN_BUFFER_INFO>::uninit();
                if GetConsoleScreenBufferInfo(handle, info.as_mut_ptr()).is_ok() {
                    let info = info.assume_init();
                    let width = info.srWindow.Right - info.srWindow.Left + 1;
                    if width > 0 {
                        return width as usize;
                    }
                }
            }
        }
    }

    80
}
