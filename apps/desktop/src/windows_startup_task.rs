//! Current-user, least-privilege interactive logon task. No shell or stored password.
use std::path::Path;
use windows::core::{Interface, BSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL, VARIANT_BOOL};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::TaskScheduler::*;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::System::Variant::VARIANT;

const MARKER: &str = "ToolHub current-user login startup v1";
const ARGUMENTS: &str = "--autostart";
const DELAY: &str = "PT15S";

type Result<T> = std::result::Result<T, String>;
fn fail(e: windows::core::Error) -> String {
    format!("Windows 登录任务：{e}")
}

struct Apartment(bool);
impl Apartment {
    fn enter() -> Result<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            Ok(Self(true))
        }
        // Tauri may already have initialized this worker in a different apartment.
        else if hr.0 as u32 == 0x80010106 {
            Ok(Self(false))
        } else {
            Err(fail(hr.into()))
        }
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() }
        }
    }
}

fn current_sid() -> Result<String> {
    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    unsafe {
        let mut handle = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle).map_err(fail)?;
        let token = Token(handle);
        let mut bytes = 0;
        let _ = GetTokenInformation(token.0, TokenUser, None, 0, &mut bytes);
        if bytes == 0 || bytes > 65536 {
            return Err("无法读取当前用户身份".into());
        }
        // TOKEN_USER needs pointer alignment, which a Vec<u8> does not guarantee.
        let mut data = vec![0u64; (bytes as usize).div_ceil(8)];
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(data.as_mut_ptr().cast()),
            bytes,
            &mut bytes,
        )
        .map_err(fail)?;
        let user = &*data.as_ptr().cast::<TOKEN_USER>();
        let mut sid = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut sid).map_err(fail)?;
        let text = sid.to_string().map_err(|e| e.to_string());
        let _ = LocalFree(Some(HLOCAL(sid.0.cast())));
        text
    }
}

fn account_sid(value: &str) -> Result<String> {
    // Scheduler normalizes logon trigger SIDs to DOMAIN\User, while principals retain SIDs.
    if value.starts_with("S-1-") {
        return Ok(value.to_string());
    }
    use windows::core::PCWSTR;
    use windows::Win32::Security::{LookupAccountNameW, PSID, SID_NAME_USE};
    let account: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let mut sid_bytes = 0;
        let mut domain_chars = 0;
        let mut kind = SID_NAME_USE::default();
        let _ = LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(account.as_ptr()),
            None,
            &mut sid_bytes,
            None,
            &mut domain_chars,
            &mut kind,
        );
        if sid_bytes == 0 || sid_bytes > 65536 || domain_chars > 65536 {
            return Err("无法校验登录任务的用户身份".into());
        }
        let mut sid_data = vec![0u64; (sid_bytes as usize).div_ceil(8)];
        let mut domain = vec![0u16; domain_chars as usize + 1];
        let sid = PSID(sid_data.as_mut_ptr().cast());
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(account.as_ptr()),
            Some(sid),
            &mut sid_bytes,
            Some(PWSTR(domain.as_mut_ptr())),
            &mut domain_chars,
            &mut kind,
        )
        .map_err(fail)?;
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(sid, &mut text).map_err(fail)?;
        let result = text.to_string().map_err(|e| e.to_string());
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        result
    }
}

// Field order ensures COM interfaces are released before the apartment.
struct Scheduler {
    folder: ITaskFolder,
    sid: String,
    _apartment: Apartment,
}
impl Scheduler {
    fn connect() -> Result<Self> {
        let apartment = Apartment::enter()?;
        let sid = current_sid()?;
        let folder = unsafe {
            let service: ITaskService =
                CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER).map_err(fail)?;
            let empty = VARIANT::default();
            service
                .Connect(&empty, &empty, &empty, &empty)
                .map_err(fail)?;
            service.GetFolder(&BSTR::from("\\")).map_err(fail)?
        };
        Ok(Self {
            folder,
            sid,
            _apartment: apartment,
        })
    }
    fn name(&self) -> String {
        format!("ToolHub-Login-{}", self.sid)
    }
    fn find(&self, name: &str) -> Result<Option<IRegisteredTask>> {
        match unsafe { self.folder.GetTask(&BSTR::from(name)) } {
            Ok(task) => Ok(Some(task)),
            Err(e) if e.code().0 as u32 == 0x80070002 => Ok(None),
            Err(e) => Err(fail(e)),
        }
    }
    fn require_owned(&self, task: &IRegisteredTask) -> Result<()> {
        unsafe {
            let def = task.Definition().map_err(fail)?;
            let mut description = BSTR::new();
            def.RegistrationInfo()
                .map_err(fail)?
                .Description(&mut description)
                .map_err(fail)?;
            let mut owner = BSTR::new();
            def.Principal()
                .map_err(fail)?
                .UserId(&mut owner)
                .map_err(fail)?;
            if description != MARKER || account_sid(&owner.to_string())? != self.sid {
                return Err("同名登录任务不属于当前用户的 ToolHub，未修改该任务".into());
            }
            Ok(())
        }
    }
    fn register(&self, name: &str, executable: &Path, enabled: bool) -> Result<IRegisteredTask> {
        if let Some(task) = self.find(name)? {
            self.require_owned(&task)?;
        }
        let xml = task_xml(executable, &self.sid, enabled)?;
        unsafe {
            self.folder
                .RegisterTask(
                    &BSTR::from(name),
                    &BSTR::from(xml),
                    TASK_CREATE_OR_UPDATE.0,
                    &VARIANT::from(self.sid.as_str()),
                    &VARIANT::default(),
                    TASK_LOGON_INTERACTIVE_TOKEN,
                    &VARIANT::default(),
                )
                .map_err(fail)
        }
    }
    fn delete(&self, name: &str) -> Result<()> {
        if let Some(task) = self.find(name)? {
            self.require_owned(&task)?;
            unsafe {
                self.folder.DeleteTask(&BSTR::from(name), 0).map_err(fail)?;
            }
        }
        Ok(())
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn path_text(path: &Path) -> Result<String> {
    let text = path.to_str().ok_or("应用路径包含无法表示的字符")?;
    if !path.is_absolute() || text.chars().any(|c| c < ' ' || c == '"') {
        return Err("自启动需要有效的应用绝对路径".into());
    }
    Ok(text.strip_prefix(r"\\?\").unwrap_or(text).to_string())
}
fn task_xml(executable: &Path, sid: &str, enabled: bool) -> Result<String> {
    let exe = xml_escape(&path_text(executable)?);
    let cwd = xml_escape(&path_text(executable.parent().ok_or("应用目录不可用")?)?);
    let sid = xml_escape(sid);
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>{MARKER}</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{sid}</UserId><Delay>{DELAY}</Delay></LogonTrigger></Triggers>
  <Principals><Principal id="ToolHubUser"><UserId>{sid}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <StartWhenAvailable>true</StartWhenAvailable><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>{enabled}</Enabled>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit><RestartOnFailure><Interval>PT1M</Interval><Count>3</Count></RestartOnFailure>
  </Settings>
  <Actions Context="ToolHubUser"><Exec><Command>{exe}</Command><Arguments>{ARGUMENTS}</Arguments><WorkingDirectory>{cwd}</WorkingDirectory></Exec></Actions>
</Task>"#
    ))
}
fn same_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .eq_ignore_ascii_case(&b.replace('/', "\\"))
}

fn matches(task: &IRegisteredTask, executable: &Path, sid: &str) -> Result<bool> {
    unsafe {
        let def = task.Definition().map_err(fail)?;
        let principal = def.Principal().map_err(fail)?;
        let mut user = BSTR::new();
        let mut logon = TASK_LOGON_TYPE::default();
        let mut level = TASK_RUNLEVEL_TYPE::default();
        principal.UserId(&mut user).map_err(fail)?;
        principal.LogonType(&mut logon).map_err(fail)?;
        principal.RunLevel(&mut level).map_err(fail)?;

        if account_sid(&user.to_string())? != sid
            || logon != TASK_LOGON_INTERACTIVE_TOKEN
            || level != TASK_RUNLEVEL_LUA
        {
            return Ok(false);
        }
        let actions = def.Actions().map_err(fail)?;
        let mut count = 0;
        actions.Count(&mut count).map_err(fail)?;
        if count != 1 {
            return Ok(false);
        }
        let action = actions.get_Item(1).map_err(fail)?;
        let Ok(exec) = action.cast::<IExecAction>() else {
            return Ok(false);
        };
        let mut path = BSTR::new();
        let mut args = BSTR::new();
        let mut cwd = BSTR::new();
        exec.Path(&mut path).map_err(fail)?;
        exec.Arguments(&mut args).map_err(fail)?;
        exec.WorkingDirectory(&mut cwd).map_err(fail)?;

        if !same_path(&path.to_string(), &path_text(executable)?)
            || args != ARGUMENTS
            || !same_path(
                &cwd.to_string(),
                &path_text(executable.parent().ok_or("应用目录不可用")?)?,
            )
        {
            return Ok(false);
        }
        let triggers = def.Triggers().map_err(fail)?;
        triggers.Count(&mut count).map_err(fail)?;
        if count != 1 {
            return Ok(false);
        }
        let trigger = triggers.get_Item(1).map_err(fail)?;
        let Ok(logon) = trigger.cast::<ILogonTrigger>() else {
            return Ok(false);
        };
        let mut enabled = VARIANT_BOOL::default();
        let mut delay = BSTR::new();
        trigger.Enabled(&mut enabled).map_err(fail)?;
        logon.UserId(&mut user).map_err(fail)?;
        logon.Delay(&mut delay).map_err(fail)?;

        Ok(enabled.as_bool() && account_sid(&user.to_string())? == sid && delay == DELAY)
    }
}

fn format_last_run(date: f64, result: u32) -> Option<String> {
    // A never-run task can return a positive sentinel DATE. DATE is local wall time,
    // not a Unix UTC timestamp; preserve the components reported by Windows.
    if result == 0x41303 || date <= 0.0 || !date.is_finite() {
        return None;
    }
    let mut time = windows::Win32::Foundation::SYSTEMTIME::default();
    if unsafe { windows::Win32::System::Variant::VariantTimeToSystemTime(date, &mut time) } == 0 {
        return None;
    }
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
    ))
}

pub(super) struct TaskStatus {
    pub enabled: bool,
    pub command_matches: bool,
    pub last_run_local: Option<String>,
    pub result: String,
}
pub(super) fn status(executable: &Path) -> Result<Option<TaskStatus>> {
    let scheduler = Scheduler::connect()?;
    let Some(task) = scheduler.find(&scheduler.name())? else {
        return Ok(None);
    };
    scheduler.require_owned(&task)?;
    unsafe {
        let date = task.LastRunTime().map_err(fail)?;
        let code = task.LastTaskResult().map_err(fail)? as u32;
        let state = task.State().map_err(fail)?;
        let result = if state == TASK_STATE_RUNNING {
            "应用正在运行".into()
        } else if date <= 0.0 || code == 0x41303 {
            "尚未执行，等待下次 Windows 登录".into()
        } else if code == 0 {
            "上次运行已正常退出".into()
        } else {
            format!("上次运行结果：0x{code:08X}")
        };
        Ok(Some(TaskStatus {
            enabled: task.Enabled().map_err(fail)?.as_bool(),
            command_matches: matches(&task, executable, &scheduler.sid)?,
            last_run_local: format_last_run(date, code),
            result,
        }))
    }
}
pub(super) fn set(executable: &Path, enabled: bool) -> Result<()> {
    let scheduler = Scheduler::connect()?;
    if enabled {
        let task = scheduler.register(&scheduler.name(), executable, true)?;
        if !unsafe { task.Enabled().map_err(fail)?.as_bool() }
            || !matches(&task, executable, &scheduler.sid)?
        {
            return Err("登录任务已登记，但配置校验失败，请重试修复".into());
        }
    } else {
        scheduler.delete(&scheduler.name())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_run_preserves_wall_time_and_hides_never_run_sentinel() {
        assert_eq!(
            format_last_run(25569.5, 0).as_deref(),
            Some("1970-01-01 12:00:00")
        );
        assert_eq!(format_last_run(36500.0, 0x41303), None);
        assert_eq!(format_last_run(0.0, 0), None);
    }
    #[test]
    fn disabled_task_roundtrip_handles_unicode_and_preserves_other_task() {
        let scheduler = Scheduler::connect().unwrap();
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let names = [
            format!("ToolHub-Test-{suffix}"),
            format!("ToolHub-Test-Other-{suffix}"),
        ];
        struct Cleanup<'a>(&'a Scheduler, [String; 2]);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                for name in &self.1 {
                    let _ = self.0.delete(name);
                }
            }
        }
        let _cleanup = Cleanup(&scheduler, names.clone());
        let exe = Path::new(r"C:\带 空格 & '测试\ToolHub.exe");
        let other = scheduler.register(&names[1], exe, false).unwrap();
        let task = scheduler.register(&names[0], exe, false).unwrap();
        assert!(!unsafe { task.Enabled().unwrap().as_bool() });
        assert!(
            matches(&task, exe, &scheduler.sid).unwrap(),
            "registered XML: {}",
            unsafe { task.Xml().unwrap() }
        );
        assert!(!matches(
            &task,
            Path::new(r"C:\Different\ToolHub.exe"),
            &scheduler.sid
        )
        .unwrap());
        let settings = unsafe { task.Definition().unwrap().Settings().unwrap() };
        let mut count = 0;
        let mut interval = BSTR::new();
        unsafe {
            settings.RestartCount(&mut count).unwrap();
            settings.RestartInterval(&mut interval).unwrap();
        }
        assert_eq!(count, 3);
        assert_eq!(interval.to_string(), "PT1M");
        scheduler.delete(&names[0]).unwrap();
        scheduler.delete(&names[0]).unwrap();
        assert!(scheduler.find(&names[0]).unwrap().is_none());
        assert!(scheduler.find(&names[1]).unwrap().is_some());
        assert!(!unsafe { other.Enabled().unwrap().as_bool() });
    }
    #[test]
    fn foreign_task_is_not_overwritten_or_deleted() {
        let scheduler = Scheduler::connect().unwrap();
        let name = format!("ToolHub-Foreign-Test-{}", std::process::id());
        struct Cleanup<'a>(&'a Scheduler, String);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                unsafe {
                    let _ = self.0.folder.DeleteTask(&BSTR::from(self.1.as_str()), 0);
                }
            }
        }
        let _cleanup = Cleanup(&scheduler, name.clone());
        let exe = Path::new(r"C:\ToolHub.exe");
        let xml = task_xml(exe, &scheduler.sid, false)
            .unwrap()
            .replace(MARKER, "Other application test");
        unsafe {
            scheduler
                .folder
                .RegisterTask(
                    &BSTR::from(name.as_str()),
                    &BSTR::from(xml),
                    TASK_CREATE_OR_UPDATE.0,
                    &VARIANT::from(scheduler.sid.as_str()),
                    &VARIANT::default(),
                    TASK_LOGON_INTERACTIVE_TOKEN,
                    &VARIANT::default(),
                )
                .unwrap();
        }
        assert!(scheduler.register(&name, exe, false).is_err());
        assert!(scheduler.delete(&name).is_err());
        assert!(scheduler.find(&name).unwrap().is_some());
    }
}
