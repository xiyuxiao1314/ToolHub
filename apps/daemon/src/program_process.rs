//! Windows process creation with fresh console handles and a sanitized environment block.
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

pub(super) fn spawn_console_helper(data: &str, entry: &ProgramEntry) -> Result<u32, String> {
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
        command.push_str(" --program-console ");
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
        | if shortcut {
            CREATE_NO_WINDOW
        } else if gui {
            0
        } else {
            CREATE_NEW_CONSOLE
        };
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // No STARTF_USESTDHANDLES: Windows assigns the new console's own input/output handles.
    let success = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            env.as_ptr().cast(),
            cwd.as_ptr(),
            &startup,
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
