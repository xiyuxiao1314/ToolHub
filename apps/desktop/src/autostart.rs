//! Explicit settings actions manage only ToolHub's current-user login registration.
#[cfg(windows)]
use crate::startup_diagnostics;
use crate::startup_diagnostics::StartupRecord;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct StartupStatus {
    supported: bool,
    enabled: bool,
    registered: bool,
    command_matches: bool,
    backend: &'static str,
    last_run_local: Option<String>,
    task_result: Option<String>,
    last_startup: Option<StartupRecord>,
}

fn launch_command(executable: &Path) -> Result<String, String> {
    let path = executable.to_str().ok_or("应用路径包含无法表示的字符")?;
    if !executable.is_absolute() || path.contains(['"', '\0']) {
        return Err("自启动需要有效的应用绝对路径".into());
    }
    Ok(format!("\"{path}\""))
}

fn status_inner() -> Result<StartupStatus, String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let legacy =
            windows_registry::read(windows_registry::RUN_KEY, "ToolHub")?.filter(|s| !s.is_empty());
        if let Some(task) = crate::windows_startup_task::status(&exe)? {
            return Ok(StartupStatus {
                supported: true,
                registered: true,
                enabled: task.enabled || legacy.is_some(),
                command_matches: task.command_matches && legacy.is_none(),
                backend: "task_scheduler",
                last_run_local: task.last_run_local,
                task_result: Some(task.result),
                last_startup: startup_diagnostics::last(),
            });
        }
        Ok(StartupStatus {
            supported: true,
            registered: legacy.is_some(),
            enabled: legacy.is_some(),
            command_matches: legacy.as_ref() == Some(&launch_command(&exe)?),
            backend: if legacy.is_some() {
                "legacy_run"
            } else {
                "none"
            },
            last_run_local: None,
            task_result: None,
            last_startup: startup_diagnostics::last(),
        })
    }
    #[cfg(not(windows))]
    Ok(StartupStatus {
        supported: false,
        enabled: false,
        registered: false,
        command_matches: false,
        backend: "none",
        last_run_local: None,
        task_result: None,
        last_startup: None,
    })
}

#[tauri::command]
pub async fn autostart_status() -> Result<StartupStatus, String> {
    tauri::async_runtime::spawn_blocking(status_inner)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn autostart_set(enabled: bool) -> Result<StartupStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            // Register and validate first. A scheduler failure must leave the old Run entry intact.
            crate::windows_startup_task::set(&exe, enabled)?;
            windows_registry::write(windows_registry::RUN_KEY, "ToolHub", None)?;
            status_inner()
        }
        #[cfg(not(windows))]
        {
            let _ = enabled;
            Err("开机自启动暂仅支持 Windows".into())
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(windows)]
mod windows_registry {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR,
    };
    use windows::Win32::System::Registry::*;

    pub(super) const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn checked(code: WIN32_ERROR) -> Result<(), String> {
        if code == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!(
                "Windows 自启动设置失败：{}",
                std::io::Error::from_raw_os_error(code.0 as i32)
            ))
        }
    }
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }

    pub(super) fn read(key: &str, name: &str) -> Result<Option<String>, String> {
        let key = wide(key);
        let name = wide(name);
        // Retry boundedly if another process changes the value between the size and data reads.
        for _ in 0..3 {
            let mut bytes = 0;
            let code = unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key.as_ptr()),
                    PCWSTR(name.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    None,
                    Some(&mut bytes),
                )
            };
            if code == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            checked(code)?;
            if bytes > 65536 || bytes % 2 != 0 {
                return Err("自启动注册值格式无效".into());
            }
            let mut data = vec![0u16; bytes as usize / 2 + 1];
            bytes = (data.len() * 2) as u32;
            let code = unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key.as_ptr()),
                    PCWSTR(name.as_ptr()),
                    RRF_RT_REG_SZ,
                    None,
                    Some(data.as_mut_ptr().cast()),
                    Some(&mut bytes),
                )
            };
            if code == ERROR_MORE_DATA {
                continue;
            }
            if code == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            checked(code)?;
            let end = data.iter().position(|c| *c == 0).unwrap_or(data.len());
            return String::from_utf16(&data[..end])
                .map(Some)
                .map_err(|e| e.to_string());
        }
        Err("自启动配置正在被其他程序修改，请重试".into())
    }

    pub(super) fn write(key: &str, name: &str, command: Option<&str>) -> Result<(), String> {
        let key = wide(key);
        let name = wide(name);
        let mut handle = HKEY::default();
        let code = unsafe {
            if command.is_some() {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key.as_ptr()),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_SET_VALUE,
                    None,
                    &mut handle,
                    None,
                )
            } else {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(key.as_ptr()),
                    None,
                    KEY_SET_VALUE,
                    &mut handle,
                )
            }
        };
        if command.is_none() && code == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        checked(code)?;
        let handle = Key(handle);
        if let Some(command) = command {
            let bytes: Vec<u8> = wide(command).iter().flat_map(|c| c.to_le_bytes()).collect();
            checked(unsafe {
                RegSetValueExW(handle.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(&bytes))
            })
        } else {
            let code = unsafe { RegDeleteValueW(handle.0, PCWSTR(name.as_ptr())) };
            if code == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                checked(code)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn isolated_registry_roundtrip_preserves_other_values() {
            let path = format!(
                r"Software\ToolHub\Tests\Startup-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            struct Cleanup(String);
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    let path = wide(&self.0);
                    unsafe {
                        let _ = RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()));
                    }
                }
            }
            let _cleanup = Cleanup(path.clone());
            assert_eq!(read(&path, "ToolHub").unwrap(), None);
            write(&path, "Other", Some("preserve")).unwrap();
            let command =
                super::super::launch_command(Path::new(r"C:\带 空格\ToolHub.exe")).unwrap();
            write(&path, "ToolHub", Some(&command)).unwrap();
            assert_eq!(
                read(&path, "ToolHub").unwrap().as_deref(),
                Some(command.as_str())
            );
            write(&path, "ToolHub", None).unwrap();
            write(&path, "ToolHub", None).unwrap();
            assert_eq!(read(&path, "ToolHub").unwrap(), None);
            assert_eq!(read(&path, "Other").unwrap().as_deref(), Some("preserve"));
        }
        use std::path::Path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_relative_command_paths() {
        assert!(launch_command(Path::new("toolhub-desktop.exe")).is_err());
    }
}
