pub fn format_bytes(kb: u64) -> String {
    let gb = kb as f64 / 1024.0 / 1024.0;
    format!("{:.2} GB", gb)
}

pub fn calc_percent(used: u64, total: u64) -> u8 {
    if total == 0 {
        0
    } else {
        ((used as f64 / total as f64) * 100.0).round() as u8
    }
}

pub fn parse_df_line(line: &str) -> Option<(u64, u64)> {
    let columns: Vec<&str> = line.split_whitespace().collect();
    if columns.len() < 4 {
        return None;
    }
    let total_kb: u64 = columns[1].parse().ok()?;
    let avail_kb: u64 = columns[3].parse().ok()?;
    let used_kb = total_kb.saturating_sub(avail_kb);
    Some((total_kb, used_kb))
}
