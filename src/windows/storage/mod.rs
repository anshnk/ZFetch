use crate::common::storage::{calc_percent, format_bytes};
use crate::system::StorageInfo;

pub async fn collect() -> Vec<StorageInfo> {
    use tokio::process::Command;

    let mut storage = Vec::new();

    if let Ok(output) = Command::new("wmic")
        .args(&[
            "logicaldisk",
            "get",
            "name,size,freespace,filesystem,drivetype",
        ])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines().skip(1) {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 5 {
                continue;
            }

            // drivetype 3 == local disk
            if columns[4] != "3" {
                continue;
            }

            let name = columns[0].trim().to_string();
            if name.is_empty() {
                continue;
            }

            let total: u64 = columns[1].parse().unwrap_or(0);
            let free: u64 = columns[2].parse().unwrap_or(0);
            let used = total.saturating_sub(free);

            storage.push(StorageInfo {
                name: format!("{}/", name.to_uppercase()),
                total: format_bytes(total / 1024),
                used: format_bytes(used / 1024),
                percent: calc_percent(used, total),
                fs_type: columns[3].to_string(),
                readonly: false,
            });
        }
    }

    storage
}
