//! Native selection is completed before the frontend receives an editable program draft.
#[tauri::command]
pub async fn pick_program_path(
    window: tauri::WebviewWindow,
    directory: bool,
) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        tauri::async_runtime::spawn_blocking(move || pick(owner, directory))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = (window, directory);
        Err("当前程序选择器仅支持 Windows".into())
    }
}

#[cfg(windows)]
fn pick(owner: usize, directory: bool) -> Result<Option<String>, String> {
    use windows::{
        core::w,
        Win32::{
            Foundation::HWND,
            System::Com::*,
            UI::Shell::{Common::COMDLG_FILTERSPEC, *},
        },
    };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() }
        }
    }
    // This dedicated worker owns the COM apartment for the lifetime of the dialog.
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|e| e.to_string())?;
        let _apartment = Apartment;
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| e.to_string())?;
        let options = FOS_FORCEFILESYSTEM
            | FOS_PATHMUSTEXIST
            | FOS_NOCHANGEDIR
            | if directory {
                FOS_PICKFOLDERS
            } else {
                FOS_FILEMUSTEXIST | FOS_NODEREFERENCELINKS
            };
        dialog.SetOptions(options).map_err(|e| e.to_string())?;
        dialog
            .SetTitle(if directory {
                w!("选择命令工作目录")
            } else {
                w!("选择程序启动文件")
            })
            .map_err(|e| e.to_string())?;
        if !directory {
            dialog
                .SetFileTypes(&[COMDLG_FILTERSPEC {
                    pszName: w!("程序与启动文件"),
                    pszSpec: w!("*.exe;*.bat;*.cmd;*.lnk;*.ps1"),
                }])
                .map_err(|e| e.to_string())?;
        }
        if let Err(e) = dialog.Show(Some(HWND(owner as *mut _))) {
            if e.code().0 == 0x800704c7u32 as i32 {
                return Ok(None);
            }
            return Err(e.to_string());
        }
        let path = dialog
            .GetResult()
            .and_then(|item| item.GetDisplayName(SIGDN_FILESYSPATH))
            .map_err(|e| e.to_string())?;
        let result = path.to_string().map_err(|e| e.to_string());
        CoTaskMemFree(Some(path.0.cast()));
        result.map(Some)
    }
}
