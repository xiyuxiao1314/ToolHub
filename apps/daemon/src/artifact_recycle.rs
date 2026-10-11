//! Native recycle-only operation; never falls back to remove_file/remove_dir.
use std::path::Path;

#[cfg(windows)]
mod windows_recycle {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use windows::core::{implement, Ref, HRESULT, PCWSTR};
    use windows::Win32::Foundation::E_ACCESSDENIED;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::*;

    #[implement(IFileOperationProgressSink)]
    struct RecycleOnly {
        recycled: Arc<AtomicBool>,
    }
    // The generated method stubs are supplied below; all non-delete operations are rejected.
    impl IFileOperationProgressSink_Impl for RecycleOnly_Impl {
        fn StartOperations(&self) -> windows::core::Result<()> {
            Ok(())
        }
        fn FinishOperations(&self, result: HRESULT) -> windows::core::Result<()> {
            result.ok()
        }
        fn PreDeleteItem(&self, flags: u32, _item: Ref<IShellItem>) -> windows::core::Result<()> {
            if flags & TSF_DELETE_RECYCLE_IF_POSSIBLE.0 as u32 == 0 {
                return Err(E_ACCESSDENIED.into());
            }
            Ok(())
        }
        fn PostDeleteItem(
            &self,
            _flags: u32,
            _item: Ref<IShellItem>,
            result: HRESULT,
            new_item: Ref<IShellItem>,
        ) -> windows::core::Result<()> {
            result.ok()?;
            if new_item.is_null() {
                return Err(E_ACCESSDENIED.into());
            }
            self.recycled.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn PreRenameItem(
            &self,
            _dwflags: u32,
            _psiitem: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PostRenameItem(
            &self,
            _dwflags: u32,
            _psiitem: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
            _hrrename: windows::core::HRESULT,
            _psinewlycreated: windows::core::Ref<IShellItem>,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PreMoveItem(
            &self,
            _dwflags: u32,
            _psiitem: windows::core::Ref<IShellItem>,
            _psidestinationfolder: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PostMoveItem(
            &self,
            _dwflags: u32,
            _psiitem: windows::core::Ref<IShellItem>,
            _psidestinationfolder: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
            _hrmove: windows::core::HRESULT,
            _psinewlycreated: windows::core::Ref<IShellItem>,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PreCopyItem(
            &self,
            _dwflags: u32,
            _psiitem: windows::core::Ref<IShellItem>,
            _psidestinationfolder: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PostCopyItem(
            &self,
            _dwflags: u32,
            _psiitem: windows::core::Ref<IShellItem>,
            _psidestinationfolder: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
            _hrcopy: windows::core::HRESULT,
            _psinewlycreated: windows::core::Ref<IShellItem>,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PreNewItem(
            &self,
            _dwflags: u32,
            _psidestinationfolder: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn PostNewItem(
            &self,
            _dwflags: u32,
            _psidestinationfolder: windows::core::Ref<IShellItem>,
            _psznewname: &windows::core::PCWSTR,
            _psztemplatename: &windows::core::PCWSTR,
            _dwfileattributes: u32,
            _hrnew: windows::core::HRESULT,
            _psinewitem: windows::core::Ref<IShellItem>,
        ) -> windows::core::Result<()> {
            Err(E_ACCESSDENIED.into())
        }
        fn UpdateProgress(&self, _iworktotal: u32, _iworksofar: u32) -> windows::core::Result<()> {
            Ok(())
        }
        fn ResetTimer(&self) -> windows::core::Result<()> {
            Ok(())
        }
        fn PauseTimer(&self) -> windows::core::Result<()> {
            Ok(())
        }
        fn ResumeTimer(&self) -> windows::core::Result<()> {
            Ok(())
        }
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    pub(super) fn recycle(path: &Path) -> std::result::Result<(), String> {
        let path = path.to_string_lossy();
        let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
        if path.starts_with(r"\\") {
            return Err("Network paths cannot guarantee a local Recycle Bin".into());
        }
        let wide: Vec<u16> = std::ffi::OsStr::new(path)
            .encode_wide()
            .chain([0])
            .collect();
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                .ok()
                .map_err(|e| e.to_string())?;
            let _apartment = Apartment;
            let operation: IFileOperation =
                CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| e.to_string())?;
            operation
                .SetOperationFlags(
                    FOFX_RECYCLEONDELETE
                        | FOFX_ADDUNDORECORD
                        | FOFX_EARLYFAILURE
                        | FOF_NOERRORUI
                        | FOF_SILENT
                        | FOF_NOCONFIRMATION
                        | FOF_NO_CONNECTED_ELEMENTS
                        | FOF_NORECURSION,
                )
                .map_err(|e| e.to_string())?;
            let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
                .map_err(|e| e.to_string())?;
            let recycled = Arc::new(AtomicBool::new(false));
            let sink: IFileOperationProgressSink = RecycleOnly {
                recycled: Arc::clone(&recycled),
            }
            .into();
            operation
                .DeleteItem(&item, &sink)
                .map_err(|e| e.to_string())?;
            operation.PerformOperations().map_err(|e| e.to_string())?;
            if operation
                .GetAnyOperationsAborted()
                .map_err(|e| e.to_string())?
                .as_bool()
                || !recycled.load(Ordering::SeqCst)
            {
                return Err("Shell did not confirm moving this file to the Recycle Bin".into());
            }
        }
        Ok(())
    }
}

pub(super) fn recycle(path: &Path) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        windows_recycle::recycle(path)
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};
        let script="on run argv\n tell application \"Finder\"\n delete (POSIX file (item 1 of argv) as alias)\n end tell\nend run";
        let mut child = Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(script)
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                return if status.success() {
                    Ok(())
                } else {
                    Err("Finder Trash operation failed; no permanent fallback".into())
                };
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "Finder Trash deadline; inspect source and Trash before retrying".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = path;
        Err("System Trash is not supported on this platform".into())
    }
}
