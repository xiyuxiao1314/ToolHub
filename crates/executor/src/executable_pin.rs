//! Keep executable identity alive from authorization through process launch.
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ExecutablePin {
    file: File,
    path: PathBuf,
}
impl ExecutablePin {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let path = path.canonicalize()?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // Permit loader reads, but deny all writes, rename and deletion while held.
            options.share_mode(1);
        }
        let file = options.open(&path)?;
        if !file.metadata()?.is_file() {
            return Err(std::io::Error::other("executable is not a file"));
        }
        Ok(Self { file, path })
    }
    pub fn canonical_path(&self) -> &Path {
        &self.path
    }
    pub fn hash(&self) -> std::io::Result<String> {
        let mut file = self.file.try_clone()?;
        file.seek(SeekFrom::Start(0))?;
        let mut hash = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = file.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            hash.update(&chunk[..n]);
        }
        Ok(hex::encode(hash.finalize()))
    }
    pub(crate) fn command(&self) -> std::io::Result<Command> {
        #[cfg(windows)]
        {
            Ok(Command::new(&self.path))
        }
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            use std::os::unix::process::CommandExt;
            let fd = self.file.as_raw_fd();
            #[cfg(target_os = "linux")]
            let path = format!("/proc/self/fd/{fd}");
            #[cfg(not(target_os = "linux"))]
            let path = format!("/dev/fd/{fd}");
            let mut command = Command::new(path);
            // SAFETY: fcntl is async-signal-safe. Only the forked child clears
            // CLOEXEC on its own inherited descriptor. The parent retains its flag.
            unsafe {
                command.pre_exec(move || {
                    extern "C" {
                        fn fcntl(fd: i32, command: i32, ...) -> i32;
                    }
                    let flags = fcntl(fd, 1); // F_GETFD
                    if flags < 0 || fcntl(fd, 2, flags & !1) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            Ok(command)
        }
    }
}
