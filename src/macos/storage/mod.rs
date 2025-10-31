use crate::common::storage::{calc_percent, format_bytes, parse_df_line};
use crate::system::StorageInfo;

pub async fn collect() -> Vec<StorageInfo> {
    use tokio::process::Command;

    let fs_type = Command::new("stat")
        .args(&["-f", "%T", "/"])
        .output()
        .await
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|fs| fs.trim().to_string())
        .unwrap_or_else(|| "apfs".to_string());

    let mut storage = Vec::new();

    if let Ok(output) = Command::new("df").args(&["-k", "/"]).output().await {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines().skip(1) {
            if !line.ends_with(" /") {
                continue;
            }

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
