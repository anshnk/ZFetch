use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use windows::{Win32::Devices::DeviceAndDriverInstallation::*, Win32::Foundation::*};

pub fn build_gpu_task() -> Option<Pin<Box<dyn Future<Output = Vec<String>> + Send>>> {
    Some(Box::pin(async { enumerate_gpus() }))
}

fn enumerate_gpus() -> Vec<String> {
    let mut names = BTreeSet::new();

    unsafe {
        if let Ok(hdev) =
            SetupDiGetClassDevsW(Some(&GUID_DEVCLASS_DISPLAY), None, None, DIGCF_PRESENT)
        {
            let mut index = 0;
            loop {
                let mut did = SP_DEVINFO_DATA {
                    cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                    ..Default::default()
                };
                if SetupDiEnumDeviceInfo(hdev, index, &mut did).is_err() {
                    break;
                }
                index += 1;

                let mut buffer = [0u16; 256];
                if SetupDiGetDeviceRegistryPropertyW(
                    hdev,
                    &did,
                    SPDRP_DEVICEDESC,
                    None,
                    Some(std::slice::from_raw_parts_mut(
                        buffer.as_mut_ptr() as *mut u8,
                        buffer.len() * 2,
                    )),
                    None,
                )
                .is_ok()
                {
                    if let Some(len) = buffer.iter().position(|&c| c == 0) {
                        let name = String::from_utf16_lossy(&buffer[..len]).trim().to_string();
                        if !name.is_empty() {
                            names.insert(name);
                        }
                    }
                }
            }
            let _ = SetupDiDestroyDeviceInfoList(hdev);
        }
    }

    names.into_iter().collect()
}
