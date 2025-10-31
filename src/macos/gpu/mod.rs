use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use serde_json::Value;
use tokio::process::Command;

pub fn build_gpu_task() -> Option<Pin<Box<dyn Future<Output = String> + Send>>> {
    Some(Box::pin(async {
        let gpus = detect_gpu_iokit();
        if !gpus.is_empty() {
            return gpus.into_iter().collect::<Vec<_>>().join(", ");
        }

        if let Ok(output) = Command::new("ioreg")
            .args(&["-r", "-c", "IOPCIDevice"])
            .output()
            .await
        {
            if let Some(model) = parse_ioreg_output(&output.stdout) {
                return model;
            }
        }

        if let Ok(output) = Command::new("system_profiler")
            .args(&["SPDisplaysDataType", "-json"])
            .output()
            .await
        {
            if let Some(model) = parse_system_profiler_output(&output.stdout) {
                return model;
            }
        }

        "Unknown".to_string()
    }))
}

fn parse_ioreg_output(stdout: &[u8]) -> Option<String> {
    let stdout = String::from_utf8_lossy(stdout);
    stdout.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.contains("model")
            && (value.contains("Apple") || value.contains("display") || value.contains("GPU"))
        {
            Some(value.trim().replace('"', ""))
        } else {
            None
        }
    })
}

fn parse_system_profiler_output(stdout: &[u8]) -> Option<String> {
    let json: Value = serde_json::from_slice(stdout).ok()?;
    let gpus = json.get("SPDisplaysDataType")?.as_array()?;
    let gpu = gpus.first()?;

    let model = gpu
        .get("sppci_model")
        .and_then(|value| value.as_str())
        .unwrap_or("Unknown");
    let cores = gpu
        .get("spdisplays_gpu_core_count")
        .and_then(|value| value.as_u64());
    let freq = gpu
        .get("spdisplays_gpu_core_clock")
        .and_then(|value| value.as_str());

    let mut details = model.to_string();
    if let Some(cores) = cores {
        details.push_str(&format!(" ({} cores", cores));
        if let Some(freq) = freq {
            details.push_str(&format!(", {})", freq));
        } else {
            details.push(')');
        }
    }
    Some(details)
}

fn detect_gpu_iokit() -> Vec<String> {
    use core_foundation::base::{CFGetTypeID, CFRelease, CFType, TCFType, ToVoid};
    use core_foundation::data::{CFData, CFDataGetTypeID};
    use core_foundation::string::{CFString, CFStringGetTypeID};
    use io_kit_sys::types::io_iterator_t;
    use io_kit_sys::*;

    let mut gpus = BTreeSet::new();

    unsafe {
        let matching_dict = IOServiceMatching(b"IOAccelerator\0".as_ptr() as *const i8);
        if matching_dict.is_null() {
            return gpus.into_iter().collect();
        }

        let mut iter: io_iterator_t = 0;
        if IOServiceGetMatchingServices(0, matching_dict, &mut iter) != 0 {
            return gpus.into_iter().collect();
        }

        loop {
            let service = IOIteratorNext(iter);
            if service == 0 {
                break;
            }

            let io_name_key = CFString::new("IOName");
            let cf_name = IORegistryEntryCreateCFProperty(
                service,
                io_name_key.as_concrete_TypeRef(),
                std::ptr::null(),
                0,
            );

            if !cf_name.is_null() {
                let cf_str = CFString::wrap_under_create_rule(cf_name as *const _);
                let name = cf_str.to_string();
                if !name.is_empty() {
                    gpus.insert(name);
                }
                CFRelease(<*const _ as ToVoid<CFType>>::to_void(&cf_name));
            }

            let model_key = CFString::new("model");
            let cf_model = IORegistryEntryCreateCFProperty(
                service,
                model_key.as_concrete_TypeRef(),
                std::ptr::null(),
                0,
            );

            if !cf_model.is_null() {
                let type_id = CFGetTypeID(cf_model);

                if type_id == CFDataGetTypeID() {
                    let cf_data = CFData::wrap_under_create_rule(cf_model as *const _);
                    if let Ok(model_str) = std::str::from_utf8(cf_data.bytes()) {
                        let model_str = model_str.trim_matches(char::from(0)).to_string();
                        if !model_str.is_empty() {
                            gpus.insert(model_str);
                        }
                    }
                } else if type_id == CFStringGetTypeID() {
                    let cf_str = CFString::wrap_under_create_rule(cf_model as *const _);
                    let model_str = cf_str.to_string();
                    if !model_str.is_empty() {
                        gpus.insert(model_str);
                    }
                }
                CFRelease(<*const _ as ToVoid<CFType>>::to_void(&cf_model));
            }

            IOObjectRelease(service);
        }

        IOObjectRelease(iter);
    }

    gpus.into_iter().collect()
}
