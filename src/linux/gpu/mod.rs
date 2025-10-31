use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::common::gpu::{detect_opengl_renderer, detect_vulkan_gpu};

pub fn build_gpu_task() -> Option<Pin<Box<dyn Future<Output = String> + Send>>> {
    Some(Box::pin(async {
        if let Some(names) = read_sys_class_drm() {
            return join_names(names);
        }

        if let Ok(Ok(names)) = timeout(Duration::from_millis(350), run_lspci()).await {
            if !names.is_empty() {
                return join_names(names);
            }
        }

        if let Some(names) = read_sys_pci_devices() {
            if !names.is_empty() {
                return join_names(names);
            }
        }

        if let Some(gl_gpu) = detect_opengl_renderer() {
            return gl_gpu;
        }

        if let Some(vk_gpu) = detect_vulkan_gpu() {
            return vk_gpu;
        }

        "Unknown".to_string()
    }))
}

fn join_names(names: Vec<String>) -> String {
    if names.is_empty() {
        "Unknown".to_string()
    } else {
        names.join(", ")
    }
}

fn read_sys_class_drm() -> Option<Vec<String>> {
    let mut gpus = BTreeSet::new();

    let entries = std::fs::read_dir("/sys/class/drm").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_str()?;
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }

        let device_path = path.join("device");
        let vendor = std::fs::read_to_string(device_path.join("vendor")).ok()?;
        let device = std::fs::read_to_string(device_path.join("device")).ok()?;
        gpus.insert(format!("PCI {}:{}", vendor.trim(), device.trim()));
    }

    Some(gpus.into_iter().collect())
}

async fn run_lspci() -> std::io::Result<Vec<String>> {
    let output = Command::new("lspci").output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut gpus = BTreeSet::new();

    for line in stdout.lines() {
        if line.contains(" VGA ") || line.contains("3D controller") {
            if let Some(device) = line.split(':').last() {
                let name = device.trim();
                if !name.is_empty() {
                    gpus.insert(name.to_string());
                }
            }
        }
    }

    Ok(gpus.into_iter().collect())
}

fn read_sys_pci_devices() -> Option<Vec<String>> {
    let mut gpus = BTreeSet::new();
    let entries = std::fs::read_dir("/sys/bus/pci/devices").ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        let class = std::fs::read_to_string(path.join("class")).ok()?;
        if !class.trim().starts_with("0x03") {
            continue;
        }

        let vendor = std::fs::read_to_string(path.join("vendor")).ok()?;
        let device = std::fs::read_to_string(path.join("device")).ok()?;
        gpus.insert(format!("PCI {}:{}", vendor.trim(), device.trim()));
    }

    Some(gpus.into_iter().collect())
}
