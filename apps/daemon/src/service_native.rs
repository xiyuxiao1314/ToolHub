//! Native observations and exact-handle control. No shell, port probes or global inventory.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    mem::{size_of, zeroed},
    path::Path,
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::*,
    NetworkManagement::IpHelper::*,
    Networking::WinSock::{AF_INET, AF_INET6},
    Security::*,
    System::{JobObjects::*, ProcessStatus::*, Threading::*},
    UI::WindowsAndMessaging::*,
};

pub struct Handle(pub HANDLE);
// Owned kernel handles may move between the serialized daemon's threads.
unsafe impl Send for Handle {}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn err() -> String {
    std::io::Error::last_os_error().to_string()
}
fn ft(t: FILETIME) -> u64 {
    (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub pid: u32,
    pub created: String,
    pub executable: String,
}
pub struct Process {
    pub handle: Handle,
    pub identity: Identity,
    pub memory_bytes: Option<usize>,
}
fn token_user(process: HANDLE) -> Result<Vec<usize>, String> {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return Err(err());
        }
        let token = Handle(token);
        let mut size = 0;
        GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut size);
        if size == 0 || size > 65536 {
            return Err("token unavailable".into());
        }
        let mut data = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
        if GetTokenInformation(
            token.0,
            TokenUser,
            data.as_mut_ptr().cast(),
            size,
            &mut size,
        ) == 0
        {
            return Err(err());
        }
        Ok(data)
    }
}
pub fn process(pid: u32, terminate: bool) -> Result<Process, String> {
    process_inner(pid, terminate, false)
}
pub fn owned_process(pid: u32) -> Result<Process, String> {
    process_inner(pid, false, true)
}
fn process_inner(pid: u32, terminate: bool, owned: bool) -> Result<Process, String> {
    unsafe {
        if pid <= 4 || pid == std::process::id() {
            return Err("不允许登记系统或 ToolHub 自身进程".into());
        }
        let handle = Handle(OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION
                | PROCESS_SYNCHRONIZE
                | if terminate { PROCESS_TERMINATE } else { 0 },
            0,
            pid,
        ));
        if handle.0.is_null() {
            return Err(err());
        }
        if WaitForSingleObject(handle.0, 0) != WAIT_TIMEOUT {
            return Err("进程已经退出".into());
        }
        let ours = token_user(GetCurrentProcess())?;
        let theirs = token_user(handle.0)?;
        if EqualSid(
            (*(ours.as_ptr().cast::<TOKEN_USER>())).User.Sid,
            (*(theirs.as_ptr().cast::<TOKEN_USER>())).User.Sid,
        ) == 0
        {
            return Err("仅允许当前用户的进程".into());
        }
        let mut critical = 0;
        if IsProcessCritical(handle.0, &mut critical) == 0 || critical != 0 {
            return Err("不能管理关键系统进程".into());
        }
        let mut image = vec![0u16; 32768];
        let mut len = image.len() as u32;
        if QueryFullProcessImageNameW(handle.0, 0, image.as_mut_ptr(), &mut len) == 0 {
            return Err(err());
        }
        let executable = String::from_utf16_lossy(&image[..len as usize]);
        let name = Path::new(&executable)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if (!owned && name.starts_with("toolhub"))
            || matches!(
                name.as_str(),
                "explorer.exe"
                    | "lsass.exe"
                    | "csrss.exe"
                    | "winlogon.exe"
                    | "services.exe"
                    | "svchost.exe"
                    | "dwm.exe"
            )
        {
            return Err("不能管理系统桌面或 ToolHub 进程".into());
        }
        let mut windows_dir = vec![0u16; 32768];
        let wlen = windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW(
            windows_dir.as_mut_ptr(),
            windows_dir.len() as u32,
        );
        if wlen == 0 || wlen as usize >= windows_dir.len() {
            return Err("系统目录不可用".into());
        }
        let windows_dir = String::from_utf16_lossy(&windows_dir[..wlen as usize]);
        if !owned && Path::new(&executable.to_lowercase()).starts_with(windows_dir.to_lowercase()) {
            return Err("系统目录中的进程仅由系统管理".into());
        }
        let (mut created, mut exit, mut kernel, mut user) =
            (zeroed(), zeroed(), zeroed(), zeroed());
        if GetProcessTimes(handle.0, &mut created, &mut exit, &mut kernel, &mut user) == 0 {
            return Err(err());
        }
        let mut memory: PROCESS_MEMORY_COUNTERS = zeroed();
        memory.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let memory_bytes = if GetProcessMemoryInfo(handle.0, &mut memory, memory.cb) != 0 {
            Some(memory.WorkingSetSize)
        } else {
            None
        };
        Ok(Process {
            handle,
            identity: Identity {
                pid,
                created: ft(created).to_string(),
                executable,
            },
            memory_bytes,
        })
    }
}
pub fn exact(identity: &Identity, terminate: bool) -> Result<Process, String> {
    let p = process(identity.pid, terminate)?;
    if p.identity != *identity {
        return Err("进程身份已改变，拒绝操作复用的 PID".into());
    }
    Ok(p)
}

pub struct Job(pub Handle);
impl Job {
    pub fn create(name: &str) -> Result<Self, String> {
        unsafe {
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let h = CreateJobObjectW(null_mut(), wide.as_ptr());
            if h.is_null() {
                Err(err())
            } else if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(h);
                Err("job name collision".into())
            } else {
                Ok(Self(Handle(h)))
            }
        }
    }
    pub fn open(name: &str) -> Result<Self, String> {
        unsafe {
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let h = OpenJobObjectW(
                windows_sys::Win32::System::SystemServices::JOB_OBJECT_QUERY
                    | windows_sys::Win32::System::SystemServices::JOB_OBJECT_TERMINATE,
                0,
                wide.as_ptr(),
            );
            if h.is_null() {
                Err(err())
            } else {
                Ok(Self(Handle(h)))
            }
        }
    }
    pub fn pids(&self) -> Result<Vec<u32>, String> {
        unsafe {
            let mut data = vec![0usize; 4098];
            if QueryInformationJobObject(
                self.0 .0,
                JobObjectBasicProcessIdList,
                data.as_mut_ptr().cast(),
                (data.len() * size_of::<usize>()) as u32,
                null_mut(),
            ) == 0
            {
                return Err("服务进程组过大或不可查询".into());
            }
            let info = &*(data.as_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>());
            let count = info.NumberOfProcessIdsInList as usize;
            if count > 4096 || info.NumberOfAssignedProcesses != info.NumberOfProcessIdsInList {
                return Err("服务进程组超过限制".into());
            }
            Ok(
                std::slice::from_raw_parts(info.ProcessIdList.as_ptr(), count)
                    .iter()
                    .map(|p| *p as u32)
                    .collect(),
            )
        }
    }
    pub fn contains(&self, p: &Process) -> bool {
        unsafe {
            let mut yes = 0;
            IsProcessInJob(p.handle.0, self.0 .0, &mut yes) != 0 && yes != 0
        }
    }
    pub fn terminate(&self) -> Result<(), String> {
        // TerminateJobObject initiates asynchronous termination. Keep exact member
        // handles and wait for exit before declaring completion/releasing ports.
        let members = self
            .pids()?
            .into_iter()
            .filter_map(|pid| owned_process(pid).ok())
            .filter(|p| self.contains(p))
            .collect::<Vec<_>>();
        unsafe {
            if TerminateJobObject(self.0 .0, 1) == 0 {
                return Err(err());
            }
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        for p in members {
            let remaining = deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_millis()
                .min(5000) as u32;
            if unsafe { WaitForSingleObject(p.handle.0, remaining) } != WAIT_OBJECT_0 {
                return Err("进程仍在结束，请稍后核对；不会重叠启动".into());
            }
        }
        while !self.pids()?.is_empty() {
            if std::time::Instant::now() >= deadline {
                return Err("子进程仍在结束，请稍后核对".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }
}
// A tiny no-window keeper holds the named job through daemon upgrades/restarts.
// It grants no RPC authority and never terminates the job when it exits.
pub fn keep_job_alive(name: &str) -> Result<(), String> {
    if !name.starts_with("Local\\ToolHub.Service.") || name.len() > 96 {
        return Err("invalid job name".into());
    }
    let job = Job::open(name)?;
    let event_name: Vec<u16> = format!("{name}.Ready")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        let ready = Handle(OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr()));
        if !ready.0.is_null() {
            SetEvent(ready.0);
        }
    }

    let start = std::time::Instant::now();
    let mut seen = false;
    loop {
        let pids = job.pids()?;
        if !pids.is_empty() {
            seen = true
        } else if seen || start.elapsed() > std::time::Duration::from_secs(8) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(())
}
pub fn start_keeper(name: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let event_name: Vec<u16> = format!("{name}.Ready")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let ready = unsafe { Handle(CreateEventW(null_mut(), 1, 0, event_name.as_ptr())) };
    if ready.0.is_null() {
        return Err(err());
    }
    let mut command =
        std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    #[cfg(not(test))]
    command.args(["--service-keeper", name]);
    #[cfg(test)]
    command
        .args([
            "--exact",
            "service::services::tests::job_keeper_fixture",
            "--ignored",
        ])
        .env("TOOLHUB_TEST_KEEP_JOB", name);
    let mut child = command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    if unsafe { WaitForSingleObject(ready.0, 5000) } != WAIT_OBJECT_0 {
        return Err("服务生命周期守护启动未就绪".into());
    }
    Ok(())
}
pub fn force(p: &Process) -> Result<(), String> {
    unsafe {
        if TerminateProcess(p.handle.0, 1) == 0 {
            Err(err())
        } else if WaitForSingleObject(p.handle.0, 5000) == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err("进程仍在结束，请稍后检查".into())
        }
    }
}
pub fn graceful(p: &Process) -> Result<(), String> {
    unsafe {
        // WM_CLOSE is only sent to windows still owned by this live, identity-checked handle.
        unsafe extern "system" fn close(hwnd: HWND, param: LPARAM) -> i32 {
            let process = &*(param as *const Process);
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == process.identity.pid
                && WaitForSingleObject(process.handle.0, 0) == WAIT_TIMEOUT
            {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
            1
        }
        if EnumWindows(Some(close), p as *const Process as isize) == 0 {
            return Err(err());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Endpoint {
    pub protocol: String,
    pub address: String,
    pub port: u16,
    pub pid: u32,
}
// Bounded kernel tables; return only endpoints belonging to explicitly tracked identities.
pub fn endpoints(pids: &BTreeSet<u32>) -> Result<Vec<Endpoint>, String> {
    endpoints_selected(pids, &BTreeSet::new())
}
pub fn endpoints_for_ports(ports: &BTreeSet<u16>) -> Result<Vec<Endpoint>, String> {
    endpoints_selected(&BTreeSet::new(), ports)
}
fn endpoints_selected(
    pids: &BTreeSet<u32>,
    ports: &BTreeSet<u16>,
) -> Result<Vec<Endpoint>, String> {
    if pids.is_empty() && ports.is_empty() {
        return Ok(vec![]);
    }
    let mut result = BTreeSet::new();
    for family in [AF_INET, AF_INET6] {
        for tcp in [true, false] {
            unsafe {
                let mut size = 0u32;
                let read = |ptr, size: *mut u32| {
                    if tcp {
                        GetExtendedTcpTable(
                            ptr,
                            size,
                            0,
                            family as u32,
                            TCP_TABLE_OWNER_PID_LISTENER,
                            0,
                        )
                    } else {
                        GetExtendedUdpTable(ptr, size, 0, family as u32, UDP_TABLE_OWNER_PID, 0)
                    }
                };
                read(null_mut(), &mut size);
                let mut success = None;
                for _ in 0..3 {
                    if size < 4 || size > 8 * 1024 * 1024 {
                        return Err("端口表超过限制".into());
                    }
                    let mut data = vec![0u64; (size as usize).div_ceil(8)];
                    let code = read(data.as_mut_ptr().cast(), &mut size);
                    if code == 0 {
                        success = Some(data);
                        break;
                    }
                    if code != ERROR_INSUFFICIENT_BUFFER {
                        return Err(format!("端口表不可用: {code}"));
                    }
                }
                let data = success.ok_or("端口表更新繁忙")?;
                let bytes = std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * 8);
                let count = std::ptr::read_unaligned(bytes.as_ptr().cast::<u32>()) as usize;
                let row_size = match (tcp, family == AF_INET) {
                    (true, true) => size_of::<MIB_TCPROW_OWNER_PID>(),
                    (true, false) => size_of::<MIB_TCP6ROW_OWNER_PID>(),
                    (false, true) => size_of::<MIB_UDPROW_OWNER_PID>(),
                    (false, false) => size_of::<MIB_UDP6ROW_OWNER_PID>(),
                };
                if count > (bytes.len() - 4) / row_size {
                    return Err("端口表格式无效".into());
                }
                for i in 0..count {
                    let ptr = bytes.as_ptr().add(4 + i * row_size);
                    let (pid, port, address) = match (tcp, family == AF_INET) {
                        (true, true) => {
                            let r = std::ptr::read_unaligned(ptr.cast::<MIB_TCPROW_OWNER_PID>());
                            (
                                r.dwOwningPid,
                                r.dwLocalPort,
                                std::net::Ipv4Addr::from(r.dwLocalAddr.to_ne_bytes()).to_string(),
                            )
                        }
                        (true, false) => {
                            let r = std::ptr::read_unaligned(ptr.cast::<MIB_TCP6ROW_OWNER_PID>());
                            (
                                r.dwOwningPid,
                                r.dwLocalPort,
                                std::net::Ipv6Addr::from(r.ucLocalAddr).to_string(),
                            )
                        }
                        (false, true) => {
                            let r = std::ptr::read_unaligned(ptr.cast::<MIB_UDPROW_OWNER_PID>());
                            (
                                r.dwOwningPid,
                                r.dwLocalPort,
                                std::net::Ipv4Addr::from(r.dwLocalAddr.to_ne_bytes()).to_string(),
                            )
                        }
                        (false, false) => {
                            let r = std::ptr::read_unaligned(ptr.cast::<MIB_UDP6ROW_OWNER_PID>());
                            (
                                r.dwOwningPid,
                                r.dwLocalPort,
                                std::net::Ipv6Addr::from(r.ucLocalAddr).to_string(),
                            )
                        }
                    };
                    if pids.contains(&pid) || ports.contains(&u16::from_be(port as u16)) {
                        result.insert(Endpoint {
                            protocol: if tcp { "TCP" } else { "UDP" }.into(),
                            address,
                            port: u16::from_be(port as u16),
                            pid,
                        });
                    }
                }
            }
        }
    }
    Ok(result.into_iter().take(256).collect())
}
