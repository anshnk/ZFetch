use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::common::gpu::{detect_opengl_renderer, detect_vulkan_gpu};

pub fn build_gpu_task() -> Option<Pin<Box<dyn Future<Output = Vec<String>> + Send>>> {
    Some(Box::pin(async {
        if let Ok(Ok(names)) = timeout(Duration::from_millis(350), run_lspci()).await {
            if !names.is_empty() {
                return names;
            }
        }

        if let Some(gl_gpu) = detect_opengl_renderer() {
            return vec![gl_gpu];
        }

        if let Some(vk_gpu) = detect_vulkan_gpu() {
            return vec![vk_gpu];
        }

        Vec::new()
    }))
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
