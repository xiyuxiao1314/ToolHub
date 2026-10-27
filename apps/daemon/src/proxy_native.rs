//! Read-only current-user WinINet settings. PAC is configuration, never fetched/executed.
use serde_json::{json, Value};
use windows_sys::Win32::{Foundation::*, System::Registry::*};
fn read(key: HKEY, name: &str) -> Option<Value> {
    unsafe {
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut size = 0;
        let mut kind = 0;
        if RegGetValueW(
            key,
            std::ptr::null(),
            name.as_ptr(),
            RRF_RT_REG_DWORD | RRF_RT_REG_SZ,
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        if size > 32768 {
            return None;
        }
        let mut data = vec![0u32; (size as usize).div_ceil(4)];
        if RegGetValueW(
            key,
            std::ptr::null(),
            name.as_ptr(),
            RRF_RT_REG_DWORD | RRF_RT_REG_SZ,
            &mut kind,
            data.as_mut_ptr().cast(),
            &mut size,
        ) != ERROR_SUCCESS
        {
            return None;
        }
        if kind == REG_DWORD {
            Some(json!(data.first().copied()?))
        } else {
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(
                data.as_ptr().cast::<u16>(),
                size as usize / 2,
            ));
            Some(json!(text.trim_end_matches('\0')))
        }
    }
}
pub fn settings() -> Value {
    unsafe {
        let sub: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut key = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_READ, &mut key) != ERROR_SUCCESS {
            return json!({"available":false});
        }
        let enabled = read(key, "ProxyEnable")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            != 0;
        let server = read(key, "ProxyServer")
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        let pac = read(key, "AutoConfigURL")
            .and_then(|v| v.as_str().map(|s| !s.is_empty()))
            .unwrap_or(false);
        RegCloseKey(key);
        // Display only a bounded server list with any credentials removed. PAC URL can contain tokens: report presence only.
        let server = server
            .split(';')
            .take(8)
            .map(|s| s.rsplit_once('@').map_or(s, |(_, after)| after))
            .collect::<Vec<_>>()
            .join(";");
        json!({"available":true,"system_proxy_enabled":enabled,"server":server,"pac_configured":pac,"tun_status":"unknown","read_only":true})
    }
}
