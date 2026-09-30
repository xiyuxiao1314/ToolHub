use super::*;
#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use windows_sys::Win32::{
        Foundation::*, Security::Authorization::*, Security::*, Storage::FileSystem::*,
        System::Pipes::*, System::Threading::*,
    };
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn last() -> IpcError {
        std::io::Error::last_os_error().into()
    }
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    fn process_identity(pid: u32) -> IpcResult<PeerIdentity> {
        unsafe {
            let process = Handle(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid));
            if process.0.is_null() {
                return Err(last());
            }
            let principal = principal_for(process.0)?;
            let mut image = vec![0u16; 32768];
            let mut size = image.len() as u32;
            if QueryFullProcessImageNameW(process.0, 0, image.as_mut_ptr(), &mut size) == 0 {
                return Err(last());
            }
            Ok(PeerIdentity {
                principal,
                image: PathBuf::from(String::from_utf16_lossy(&image[..size as usize])),
            })
        }
    }
    unsafe fn principal_for(process: HANDLE) -> IpcResult<String> {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return Err(last());
        }
        let token = Handle(token);
        let mut len = 0;
        GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buffer = vec![
            0usize;
            (len as usize).div_ceil(std::mem::size_of::<usize>())
        ];
        if GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            len,
            &mut len,
        ) == 0
        {
            return Err(last());
        }
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut sid = std::ptr::null_mut();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid) == 0 {
            return Err(last());
        }
        let mut count = 0;
        while *sid.add(count) != 0 {
            count += 1;
        }
        let value = String::from_utf16_lossy(std::slice::from_raw_parts(sid, count));
        LocalFree(sid.cast());
        Ok(value)
    }
    pub fn current_principal() -> IpcResult<String> {
        unsafe { principal_for(GetCurrentProcess()) }
    }
    struct Security(PSECURITY_DESCRIPTOR);
    impl Security {
        fn user() -> IpcResult<Self> {
            unsafe {
                let sddl = wide(&format!("D:P(A;;GA;;;{})", current_principal()?));
                let mut descriptor = std::ptr::null_mut();
                if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    1,
                    &mut descriptor,
                    std::ptr::null_mut(),
                ) == 0
                {
                    return Err(last());
                }
                Ok(Self(descriptor))
            }
        }
        fn attributes(&self) -> SECURITY_ATTRIBUTES {
            SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: self.0,
                bInheritHandle: 0,
            }
        }
    }
    impl Drop for Security {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
    pub fn user_scoped_pipe_name() -> String {
        format!(
            r"\\.\pipe\toolhubd-{}-{}",
            current_principal().expect("OS identity"),
            namespace()
        )
    }
    pub fn daemon_socket_name() -> String {
        user_scoped_pipe_name()
    }
    pub struct ServiceLock(Handle);
    impl ServiceLock {
        pub fn acquire() -> IpcResult<Self> {
            unsafe {
                let security = Security::user()?;
                let name = wide(&format!(
                    "Local\\ToolHub-{}-{}",
                    current_principal()?,
                    namespace()
                ));
                let handle = CreateMutexW(&security.attributes(), 0, name.as_ptr());
                if handle.is_null() {
                    return Err(last());
                }
                let handle = Handle(handle);
                // Wait also handles a daemon that crashed while owning the mutex.
                let outcome = WaitForSingleObject(handle.0, 0);
                if outcome != WAIT_OBJECT_0 && outcome != WAIT_ABANDONED {
                    return Err(IpcError::Message("daemon already running".into()));
                }
                Ok(Self(handle))
            }
        }
    }
    impl Drop for ServiceLock {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.0 .0);
            }
        }
    }
    pub struct LocalListener {
        name: String,
    }
    impl LocalListener {
        pub fn bind() -> IpcResult<Self> {
            Ok(Self {
                name: user_scoped_pipe_name(),
            })
        }
        pub fn accept(&self) -> IpcResult<(LocalStream, PeerIdentity)> {
            let file = accept_named_pipe(&self.name)?;
            let peer = pipe_peer(&file, false)?;
            if peer.principal != current_principal()? {
                return Err(IpcError::Message("unauthorized peer".into()));
            }
            Ok((
                LocalStream {
                    file,
                    deadline: None,
                },
                peer,
            ))
        }
    }
    fn pipe_peer(file: &std::fs::File, server: bool) -> IpcResult<PeerIdentity> {
        unsafe {
            let mut pid = 0;
            let success = if server {
                GetNamedPipeServerProcessId(file.as_raw_handle(), &mut pid)
            } else {
                GetNamedPipeClientProcessId(file.as_raw_handle(), &mut pid)
            };
            if success == 0 {
                return Err(last());
            }
            process_identity(pid)
        }
    }
    pub fn accept_named_pipe(name: &str) -> IpcResult<std::fs::File> {
        unsafe {
            let security = Security::user()?;
            let handle = CreateNamedPipeW(
                wide(name).as_ptr(),
                3,
                PIPE_REJECT_REMOTE_CLIENTS,
                255,
                65536,
                65536,
                0,
                &security.attributes(),
            );
            if handle == INVALID_HANDLE_VALUE {
                return Err(last());
            }
            let file = std::fs::File::from_raw_handle(handle);
            if ConnectNamedPipe(handle, std::ptr::null_mut()) == 0
                && GetLastError() != ERROR_PIPE_CONNECTED
            {
                return Err(last());
            }
            Ok(file)
        }
    }
    pub fn create_named_pipe(name: &str) -> IpcResult<std::fs::File> {
        accept_named_pipe(name)
    }
    pub fn connect_named_pipe(name: &str) -> IpcResult<std::fs::File> {
        unsafe {
            let handle = CreateFileW(
                wide(name).as_ptr(),
                FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                std::ptr::null_mut(),
            );
            if handle == INVALID_HANDLE_VALUE {
                return Err(last());
            }
            Ok(std::fs::File::from_raw_handle(handle))
        }
    }
    pub struct LocalStream {
        file: std::fs::File,
        deadline: Option<std::time::Instant>,
    }
    impl LocalStream {
        pub fn connect(expected_image: &Path) -> IpcResult<Self> {
            let file = connect_named_pipe(&user_scoped_pipe_name())?;
            let peer = pipe_peer(&file, true)?;
            if peer.principal != current_principal()?
                || peer.image.canonicalize()? != expected_image.canonicalize()?
                || image_hash(&peer.image)? != image_hash(expected_image)?
            {
                return Err(IpcError::Message("unverified daemon endpoint".into()));
            }
            Ok(Self {
                file,
                deadline: None,
            })
        }
        pub fn set_timeout(&mut self, duration: std::time::Duration) -> IpcResult<()> {
            self.deadline = Some(std::time::Instant::now() + duration);
            Ok(())
        }
    }
    impl Read for LocalStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if let Some(deadline) = self.deadline {
                loop {
                    unsafe {
                        let mut available = 0;
                        if PeekNamedPipe(
                            self.file.as_raw_handle(),
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            &mut available,
                            std::ptr::null_mut(),
                        ) == 0
                        {
                            return Err(std::io::Error::last_os_error());
                        }
                        if available > 0 {
                            break;
                        }
                        if std::time::Instant::now() >= deadline {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "daemon response deadline",
                            ));
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
            self.file.read(buf)
        }
    }
    impl Write for LocalStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.file.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.file.flush()
        }
    }
}
#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
        net::{UnixListener, UnixStream},
    };
    pub fn current_principal() -> IpcResult<String> {
        Ok(format!("uid:{}", unsafe { libc::geteuid() }))
    }
    fn runtime_dir() -> IpcResult<PathBuf> {
        let path = std::env::temp_dir().join(format!("toolhub-{}", unsafe { libc::geteuid() }));
        match std::fs::create_dir(&path) {
            Ok(()) => std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
        let meta = std::fs::symlink_metadata(&path)?;
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            return Err(IpcError::Message("unsafe runtime directory".into()));
        }
        Ok(path)
    }
    pub fn daemon_socket_name() -> String {
        runtime_dir()
            .expect("secure runtime directory")
            .join(format!("{}.sock", namespace()))
            .to_string_lossy()
            .into_owned()
    }
    pub struct ServiceLock(std::fs::File);
    impl ServiceLock {
        pub fn acquire() -> IpcResult<Self> {
            let path = runtime_dir()?.join(format!("{}.lock", namespace()));
            let file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)?;
            if file.metadata()?.uid() != unsafe { libc::geteuid() } {
                return Err(IpcError::Message("foreign lock owner".into()));
            }
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(IpcError::Message("daemon already running".into()));
            }
            Ok(Self(file))
        }
    }
    pub struct LocalListener {
        inner: UnixListener,
        path: PathBuf,
    }
    impl LocalListener {
        pub fn bind() -> IpcResult<Self> {
            let path = PathBuf::from(daemon_socket_name());
            if path.exists() {
                let metadata = std::fs::symlink_metadata(&path)?;
                if metadata.uid() != unsafe { libc::geteuid() }
                    || !std::os::unix::fs::FileTypeExt::is_socket(&metadata.file_type())
                {
                    return Err(IpcError::Message("foreign endpoint".into()));
                }
                if UnixStream::connect(&path).is_ok() {
                    return Err(IpcError::Message("daemon already running".into()));
                }
                std::fs::remove_file(&path)?;
            }
            let inner = UnixListener::bind(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            Ok(Self { inner, path })
        }
        pub fn accept(&self) -> IpcResult<(LocalStream, PeerIdentity)> {
            let (stream, _) = self.inner.accept()?;
            let peer = peer(&stream)?;
            if peer.principal != current_principal()? {
                return Err(IpcError::Message("unauthorized peer".into()));
            }
            Ok((LocalStream(stream), peer))
        }
    }
    impl Drop for LocalListener {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
    fn peer(stream: &UnixStream) -> IpcResult<PeerIdentity> {
        #[cfg(target_os = "linux")]
        unsafe {
            let mut credential: libc::ucred = std::mem::zeroed();
            let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
            if libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credential as *mut libc::ucred).cast(),
                &mut size,
            ) != 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            return Ok(PeerIdentity {
                principal: format!("uid:{}", credential.uid),
                image: std::fs::read_link(format!("/proc/{}/exe", credential.pid))?,
            });
        }
        #[cfg(target_os = "macos")]
        unsafe {
            let mut uid = 0;
            let mut gid = 0;
            if libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut pid: libc::pid_t = 0;
            let mut size = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
            // LOCAL_PEERPID, obtained from the kernel, never a client field.
            if libc::getsockopt(
                stream.as_raw_fd(),
                0,
                2,
                (&mut pid as *mut libc::pid_t).cast(),
                &mut size,
            ) != 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut path = [0u8; 4096];
            let n = libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32);
            if n <= 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let end = path.iter().position(|c| *c == 0).unwrap_or(path.len());
            return Ok(PeerIdentity {
                principal: format!("uid:{}", uid),
                image: PathBuf::from(String::from_utf8_lossy(&path[..end]).as_ref()),
            });
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = stream;
            Err(IpcError::Message(
                "peer image verification unsupported".into(),
            ))
        }
    }
    pub struct LocalStream(UnixStream);
    impl LocalStream {
        pub fn connect(expected_image: &Path) -> IpcResult<Self> {
            let path = daemon_socket_name();
            let metadata = std::fs::symlink_metadata(&path)?;
            if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
                return Err(IpcError::Message("unsafe endpoint".into()));
            }
            let stream = UnixStream::connect(path)?;
            let identity = peer(&stream)?;
            if identity.principal != current_principal()?
                || identity.image.canonicalize()? != expected_image.canonicalize()?
                || image_hash(&identity.image)? != image_hash(expected_image)?
            {
                return Err(IpcError::Message("unverified daemon endpoint".into()));
            }
            Ok(Self(stream))
        }
        pub fn set_timeout(&mut self, duration: std::time::Duration) -> IpcResult<()> {
            self.0.set_read_timeout(Some(duration))?;
            self.0.set_write_timeout(Some(duration))?;
            Ok(())
        }
    }
    impl Read for LocalStream {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(b)
        }
    }
    impl Write for LocalStream {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.write(b)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }
}
#[cfg(unix)]
pub use unix::*;
