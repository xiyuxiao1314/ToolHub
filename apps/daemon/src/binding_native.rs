//! Bounded observation of a desktop-selected executable and exact argv prefix.
//! Matching is metadata, never process-control authority.
use super::native::{self, Handle, Process};
use serde::{Deserialize, Serialize};
use std::{
    mem::{size_of, zeroed},
    path::Path,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::*, System::Diagnostics::ToolHelp::*, UI::Shell::CommandLineToArgvW,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub executable_paths: Vec<String>,
    pub arguments: Vec<String>,
}
fn key(path: &str) -> String {
    path.trim_start_matches("\\\\?\\")
        .replace('/', "\\")
        .to_lowercase()
}
impl Binding {
    pub fn validate(&self) -> Result<(), String> {
        if self.executable_paths.is_empty()
            || self.executable_paths.len() > 4
            || self.arguments.is_empty()
            || self.arguments.len() > 16
        {
            return Err("独立进程匹配需指定 1–4 个实际执行文件和 1–16 项命令参数前缀".into());
        }
        for path in &self.executable_paths {
            if path.len() > 2048 || !Path::new(path).is_absolute() || !Path::new(path).is_file() {
                return Err("匹配执行文件不存在".into());
            }
        }
        if self
            .arguments
            .iter()
            .any(|a| a.is_empty() || a.len() > 2048 || a.contains(['\0', '\r', '\n']))
        {
            return Err("匹配参数无效".into());
        }
        Ok(())
    }
    pub fn find(&self) -> Result<Option<Process>, String> {
        unsafe {
            self.validate()?;
            let raw = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if raw == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let snapshot = Handle(raw);
            let names = self
                .executable_paths
                .iter()
                .filter_map(|p| {
                    Path::new(p)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                })
                .collect::<Vec<_>>();
            let mut entry: PROCESSENTRY32W = zeroed();
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot.0, &mut entry);
            let mut visited = 0;
            let mut candidates = 0;
            let mut found = None;
            while more != 0 {
                visited += 1;
                if visited > 32768 {
                    return Err("系统进程快照超过上限".into());
                }
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase();
                if names.contains(&name) {
                    candidates += 1;
                    if candidates > 64 {
                        return Err("匹配候选过多，请缩小范围".into());
                    }
                    if let Ok(p) = native::process(entry.th32ProcessID, false) {
                        if self
                            .executable_paths
                            .iter()
                            .any(|s| key(s) == key(&p.identity.executable))
                        {
                            if let Ok(argv) = arguments(&p) {
                                if argv.len() > self.arguments.len()
                                    && argv[1..].starts_with(&self.arguments)
                                {
                                    if found.is_some() {
                                        return Err(
                                            "有多个符合条件的独立进程，请核对；不会猜测或结束进程"
                                                .into(),
                                        );
                                    }
                                    found = Some(p)
                                }
                            }
                        }
                    }
                }
                more = Process32NextW(snapshot.0, &mut entry);
            }
            Ok(found)
        }
    }
}
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum: u16,
    buffer: *const u16,
}
#[link(name = "ntdll")]
extern "system" {
    fn NtQueryInformationProcess(
        process: HANDLE,
        class: u32,
        buffer: *mut core::ffi::c_void,
        length: u32,
        returned: *mut u32,
    ) -> i32;
}
fn arguments(p: &Process) -> Result<Vec<String>, String> {
    unsafe {
        let mut size = 0;
        NtQueryInformationProcess(p.handle.0, 60, null_mut(), 0, &mut size);
        if size < size_of::<UnicodeString>() as u32 || size > 65536 {
            return Err("无法读取有界启动参数".into());
        }
        let mut data = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
        if NtQueryInformationProcess(p.handle.0, 60, data.as_mut_ptr().cast(), size, &mut size) < 0
        {
            return Err("进程参数不可读取".into());
        }
        let text = &*data.as_ptr().cast::<UnicodeString>();
        let base = data.as_ptr() as usize;
        let start = text.buffer as usize;
        let bytes = text.length as usize;
        if bytes % 2 != 0
            || start < base
            || start
                .checked_add(bytes)
                .is_none_or(|end| end > base + data.len() * size_of::<usize>())
        {
            return Err("进程参数返回范围无效".into());
        }
        let mut wide = std::slice::from_raw_parts(text.buffer, bytes / 2).to_vec();
        wide.push(0);
        let mut count = 0;
        let argv = CommandLineToArgvW(wide.as_ptr(), &mut count);
        if argv.is_null() {
            return Err("启动参数解析失败".into());
        }
        let parsed = if !(1..=256).contains(&count) {
            Err("启动参数过多".into())
        } else {
            let mut result = Vec::new();
            for pointer in std::slice::from_raw_parts(argv, count as usize) {
                let mut len = 0;
                while len < 32768 && *pointer.add(len) != 0 {
                    len += 1
                }
                if len >= 32768 {
                    LocalFree(argv.cast());
                    return Err("启动参数过长".into());
                }
                result.push(String::from_utf16_lossy(std::slice::from_raw_parts(
                    *pointer, len,
                )))
            }
            Ok(result)
        };
        LocalFree(argv.cast());
        parsed
    }
}
