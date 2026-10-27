//! Windows program creation with explicit background/terminal mode and a sanitized environment.
use super::programs::ProgramEntry;
use std::path::Path;
use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};

fn quoted(value: &str) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for ch in value.chars() {
        if ch == '\\' {
            slashes += 1;
            continue;
        }
        if ch == '"' {
            out.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
        } else {
            out.extend(std::iter::repeat_n('\\', slashes));
        }
        slashes = 0;
        out.push(ch);
    }
    out.extend(std::iter::repeat_n('\\', slashes * 2));
    out.push('"');
    out
}

fn is_gui(path: &str) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 64];
    if file.read_exact(&mut header).is_err() || &header[..2] != b"MZ" {
        return false;
    }
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
    if offset > 1024 * 1024 || file.seek(SeekFrom::Start(offset)).is_err() {
        return false;
    }
    let mut pe = [0u8; 94];
    file.read_exact(&mut pe).is_ok()
        && &pe[..4] == b"PE\0\0"
        && u16::from_le_bytes([pe[92], pe[93]]) == 2
}

pub(super) fn spawn_helper(
    data: &str,
    entry: &ProgramEntry,
    terminal: bool,
) -> Result<u32, String> {
    spawn_helper_job(data, entry, terminal, None)
}

pub(super) fn spawn_helper_job(
    data: &str,
    entry: &ProgramEntry,
    terminal: bool,
    job: Option<windows_sys::Win32::Foundation::HANDLE>,
) -> Result<u32, String> {
    let c = &entry.candidate;
    let direct = c.kind == "file"
        && Path::new(&c.path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    let executable = if direct {
        c.path.clone()
    } else {
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into()
    };
    let mut command = quoted(&executable);
    if direct {
        for arg in &entry.args {
            command.push(' ');
            command.push_str(&quoted(arg));
        }
    } else {
        command.push_str(if terminal {
            " --program-console "
        } else {
            " --program-background "
        });
        command.push_str(&quoted(data));
    }
    let application: Vec<u16> = executable.encode_utf16().chain(Some(0)).collect();
    let mut command: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
    let cwd: Vec<u16> = c.cwd.encode_utf16().chain(Some(0)).collect();
    let env = super::programs::program_environment();
    let mut variables = env.vars.into_iter().collect::<Vec<_>>();
    variables.sort_by_key(|(name, _)| name.to_ascii_uppercase());
    let mut env: Vec<u16> = variables
        .iter()
        .flat_map(|(k, v)| format!("{k}={v}\0").encode_utf16().collect::<Vec<_>>())
        .collect();
    env.push(0);
    if env.len() == 1 {
        env.push(0);
    }
    let gui = direct && is_gui(&c.path);
    let shortcut = Path::new(&c.path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
    let flags = CREATE_UNICODE_ENVIRONMENT
        | if gui {
            0
        } else if terminal && !shortcut {
            CREATE_NEW_CONSOLE
        } else {
            CREATE_NO_WINDOW
        };
    let mut attribute_size = 0usize;
    let mut attributes = Vec::<usize>::new();
    let mut startup_ex: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    struct AttributeGuard(LPPROC_THREAD_ATTRIBUTE_LIST);
    impl Drop for AttributeGuard {
        fn drop(&mut self) {
            unsafe {
                DeleteProcThreadAttributeList(self.0);
            }
        }
    }
    let mut guard = None;
    let mut job_handle = job.unwrap_or(std::ptr::null_mut());
    if job.is_some() {
        unsafe {
            InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attribute_size);
            if attribute_size == 0 || attribute_size > 65536 {
                return Err("进程组属性不可用".into());
            }
            attributes.resize(attribute_size.div_ceil(std::mem::size_of::<usize>()), 0);
            let list = attributes.as_mut_ptr().cast();
            if InitializeProcThreadAttributeList(list, 1, 0, &mut attribute_size) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            guard = Some(AttributeGuard(list));
            if UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                (&mut job_handle as *mut windows_sys::Win32::Foundation::HANDLE).cast(),
                std::mem::size_of_val(&job_handle),
                std::ptr::null_mut(),
                std::ptr::null(),
            ) == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            startup_ex.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
            startup_ex.lpAttributeList = list;
        }
    }
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // Do not inherit the daemon handles. Terminal launches receive their own console handles.
    let success = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags
                | if job.is_some() {
                    EXTENDED_STARTUPINFO_PRESENT
                } else {
                    0
                },
            env.as_ptr().cast(),
            cwd.as_ptr(),
            if job.is_some() {
                &startup_ex.StartupInfo
            } else {
                &startup
            },
            &mut process,
        )
    };
    if success == 0 {
        return Err(format!("启动失败：{}", std::io::Error::last_os_error()));
    }
    unsafe {
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
    }
    drop(guard);
    Ok(process.dwProcessId)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_arguments_keep_quotes_and_trailing_slashes() {
        assert_eq!(quoted("a b\\"), "\"a b\\\\\"");
        assert_eq!(quoted("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quoted("x&y"), "\"x&y\"");
    }
}
