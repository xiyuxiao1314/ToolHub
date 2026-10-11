//! The user service must not retain its launching Agent's capture-pipe handles.
use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, TerminateProcess, WaitForSingleObject, CREATE_NO_WINDOW,
    INFINITE, PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
};

pub(super) struct BackgroundChild(OwnedHandle);
impl BackgroundChild {
    pub(super) fn spawn(daemon: &Path) -> io::Result<Self> {
        let application: Vec<u16> = daemon.as_os_str().encode_wide().chain([0]).collect();
        let mut command = OsString::from("\"");
        command.push(daemon);
        command.push("\" --listen");
        let mut command: Vec<u16> = command.encode_wide().chain([0]).collect();
        // No standard streams and no inheritable handles. The daemon communicates via IPC.
        let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup.dwFlags = STARTF_USESHOWWINDOW;
        startup.wShowWindow = 0;
        let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
        let created = unsafe {
            CreateProcessW(
                application.as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                CREATE_NO_WINDOW,
                std::ptr::null(),
                std::ptr::null(),
                &startup,
                &mut info,
            )
        };
        if created == 0 {
            return Err(io::Error::last_os_error());
        }
        unsafe {
            CloseHandle(info.hThread);
        }
        // Successful CreateProcess returns a uniquely owned process handle.
        Ok(Self(unsafe { OwnedHandle::from_raw_handle(info.hProcess) }))
    }
    pub(super) fn try_wait(&mut self) -> io::Result<Option<u32>> {
        match unsafe { WaitForSingleObject(self.0.as_raw_handle(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => self.exit_code().map(Some),
            _ => Err(io::Error::last_os_error()),
        }
    }
    pub(super) fn wait(&mut self) -> io::Result<u32> {
        if unsafe { WaitForSingleObject(self.0.as_raw_handle(), INFINITE) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        self.exit_code()
    }
    fn exit_code(&self) -> io::Result<u32> {
        let mut code = 0;
        if unsafe { GetExitCodeProcess(self.0.as_raw_handle(), &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(code)
    }
    pub(super) fn kill(&mut self) -> io::Result<()> {
        if unsafe { TerminateProcess(self.0.as_raw_handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
