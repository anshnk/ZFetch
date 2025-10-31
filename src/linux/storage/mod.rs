use crate::common::storage::{calc_percent, format_bytes, parse_df_line};
use crate::system::StorageInfo;

pub async fn collect() -> Vec<StorageInfo> {
    use tokio::process::Command;

    let fs_type = read_root_fs_type().unwrap_or_else(|| "unknown".to_string());
    let mut storage = Vec::new();

    if let Ok(output) = Command::new("df").args(&["-k", "/"]).output().await {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines().skip(1) {
            if let Some((total_kb, used_kb)) = parse_df_line(line) {
                storage.push(StorageInfo {
                    name: "/".to_string(),
                    total: format_bytes(total_kb),
                    used: format_bytes(used_kb),
                    percent: calc_percent(used_kb, total_kb),
                    fs_type: fs_type.clone(),
                    readonly: false,
                });
                break;
            }
        }
    }

    storage
}

fn read_root_fs_type() -> Option<String> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1] == "/" {
            return Some(parts[2].to_string());
        }
    }
    None
}
