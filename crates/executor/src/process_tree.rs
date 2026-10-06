//! Own only the process tree created for this invocation; no shell kill commands.
use std::process::{Child, Command};
#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::{io::AsRawHandle, process::CommandExt};
    type Handle = *mut c_void;
    #[repr(C)]
    struct BasicLimits {
        per_process: i64,
        per_job: i64,
        flags: u32,
        min: usize,
        max: usize,
        active: u32,
        affinity: usize,
        priority: u32,
        scheduling: u32,
    }
    #[repr(C)]
    struct IoCounters {
        read_ops: u64,
        write_ops: u64,
        other_ops: u64,
        read_bytes: u64,
        write_bytes: u64,
        other_bytes: u64,
    }
    #[repr(C)]
    struct ExtendedLimits {
        basic: BasicLimits,
        io: IoCounters,
        process_memory: usize,
        job_memory: usize,
        peak_process: usize,
        peak_job: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(job: Handle, class: u32, info: *const c_void, size: u32) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
        fn CancelSynchronousIo(thread: Handle) -> i32;
    }
    #[link(name = "ntdll")]
    extern "system" {
        fn NtResumeProcess(process: Handle) -> i32;
    }
    pub struct Tree(Handle);
    // SAFETY: the guard exclusively owns the kernel job handle. Moving ownership to
    // another thread is supported by Windows; all mutation requires &mut self.
    unsafe impl Send for Tree {}
    impl Tree {
        pub fn prepare(cmd: &mut Command) -> std::io::Result<Self> {
            // Suspension closes the spawn-to-assignment race: descendants inherit this job.
            cmd.creation_flags(0x00000004 | 0x08000000);
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Err(std::io::Error::last_os_error());
                }
                let mut info: ExtendedLimits = std::mem::zeroed();
                info.basic.flags = 0x2000; // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                if SetInformationJobObject(
                    job,
                    9,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<ExtendedLimits>() as u32,
                ) == 0
                {
                    CloseHandle(job);
                    return Err(std::io::Error::last_os_error());
                }
                Ok(Self(job))
            }
        }
        pub fn attach(&mut self, child: &mut Child) -> std::io::Result<()> {
            unsafe {
                if AssignProcessToJobObject(self.0, child.as_raw_handle()) == 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if NtResumeProcess(child.as_raw_handle()) < 0 {
                    return Err(std::io::Error::other("cannot resume owned process"));
                }
            }
            Ok(())
        }
        pub fn poll_child(
            &mut self,
            child: &mut Child,
        ) -> std::io::Result<Option<std::process::ExitStatus>> {
            child.try_wait()
        }
        pub fn terminate(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
                self.0 = std::ptr::null_mut();
            }
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            self.terminate();
        }
    }
    pub fn cancel_io<T>(thread: &std::thread::JoinHandle<T>) {
        unsafe {
            CancelSynchronousIo(thread.as_raw_handle());
        }
    }
}
#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::unix::process::CommandExt;
    extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    pub struct Tree(Option<i32>);
    impl Tree {
        pub fn prepare(cmd: &mut Command) -> std::io::Result<Self> {
            cmd.process_group(0);
            Ok(Self(None))
        }
        pub fn attach(&mut self, child: &mut Child) -> std::io::Result<()> {
            self.0 = Some(child.id() as i32);
            Ok(())
        }
        /// Observe exit without reaping. The group leader's zombie reserves its
        /// PID/PGID until descendants are terminated, preventing numeric reuse.
        pub fn poll_child(
            &mut self,
            child: &mut Child,
        ) -> std::io::Result<Option<std::process::ExitStatus>> {
            // SAFETY: initialized siginfo storage is passed to waitid for our own child.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    child.id() as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOWAIT | libc::WNOHANG,
                )
            };
            if result != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if info.si_signo == libc::SIGCHLD {
                self.terminate();
                child.try_wait()
            } else {
                Ok(None)
            }
        }
        pub fn terminate(&mut self) {
            if let Some(group) = self.0.take() {
                unsafe {
                    kill(-group, 9);
                }
            }
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            self.terminate();
        }
    }
    pub fn cancel_io<T>(_thread: &std::thread::JoinHandle<T>) {}
}
pub use platform::{cancel_io, Tree};

impl Tree {
    /// Launch a command atomically confined to an owned tree. Drop terminates any
    /// remaining descendants; callers must still bound their pipe collection.
    pub fn spawn(cmd: &mut Command) -> std::io::Result<(Child, Self)> {
        let mut tree = Self::prepare(cmd)?;
        let mut child = cmd.spawn()?;
        if let Err(error) = tree.attach(&mut child) {
            let _ = child.kill();
            tree.terminate();
            return Err(error);
        }
        Ok((child, tree))
    }
}

#[cfg(windows)]
pub fn make_pipes_interruptible(_child: &Child) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
pub fn make_pipes_interruptible(child: &Child) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    let descriptors = [
        child.stdout.as_ref().map(AsRawFd::as_raw_fd),
        child.stderr.as_ref().map(AsRawFd::as_raw_fd),
        child.stdin.as_ref().map(AsRawFd::as_raw_fd),
    ];
    for fd in descriptors.into_iter().flatten() {
        // SAFETY: descriptor belongs to our child's parent-side pipe handle.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}
