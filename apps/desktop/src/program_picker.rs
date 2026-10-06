//! Native selection is completed before the frontend receives an editable program draft.
#[tauri::command]
pub async fn pick_program_path(
    window: tauri::WebviewWindow,
    directory: bool,
) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        tauri::async_runtime::spawn_blocking(move || pick(owner, directory, false, false))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        #[cfg(target_os = "macos")]
        return tauri::async_runtime::spawn_blocking(move || {
            crate::macos::choose(
                if directory { "folder" } else { "file" },
                if directory {
                    "选择命令工作目录"
                } else {
                    "选择启动文件"
                },
            )
        })
        .await
        .map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "macos"))]
        {
            let _ = directory;
            Err("当前平台暂不支持程序选择器".into())
        }
    }
}

#[cfg(windows)]
fn pick(
    owner: usize,
    directory: bool,
    skill: bool,
    bundle: bool,
) -> Result<Option<String>, String> {
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
            .SetTitle(if bundle {
                w!("选择 ToolHub 通用能力包 JSON 文件")
            } else if skill {
                w!("选择含 skill.json 和 SKILL.md 的技能目录")
            } else if directory {
                w!("选择命令工作目录")
            } else {
                w!("选择程序启动文件")
            })
            .map_err(|e| e.to_string())?;
        if !directory {
            dialog
                .SetFileTypes(&[COMDLG_FILTERSPEC {
                    pszName: if bundle {
                        w!("ToolHub 能力包")
                    } else {
                        w!("程序与启动文件")
                    },
                    pszSpec: if bundle {
                        w!("*.toolhub-skill.json;*.json")
                    } else {
                        w!("*.exe;*.bat;*.cmd;*.lnk;*.ps1")
                    },
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

#[tauri::command]
pub async fn pick_skill_folder(window: tauri::WebviewWindow) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        tauri::async_runtime::spawn_blocking(move || pick(owner, true, true, false))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        #[cfg(target_os = "macos")]
        return tauri::async_runtime::spawn_blocking(|| {
            crate::macos::choose("folder", "选择含 skill.json 和 SKILL.md 的技能目录")
        })
        .await
        .map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "macos"))]
        Err("当前平台暂不支持 Skill 目录选择器".into())
    }
}
#[tauri::command]
pub fn builtin_skill_paths() -> Result<Vec<String>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let packaged = exe
        .parent()
        .ok_or("installation directory unavailable")?
        .join("skills/library");
    let root = if packaged.is_dir() {
        packaged
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills/library")
    };
    Ok(["media-compress", "pdf-text-ocr"]
        .iter()
        .map(|name| root.join(name).join("skill.json"))
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

pub async fn pick_bundle_file(window: tauri::WebviewWindow) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        tauri::async_runtime::spawn_blocking(move || pick(owner, false, false, true))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        #[cfg(target_os = "macos")]
        return tauri::async_runtime::spawn_blocking(|| {
            crate::macos::choose("file", "选择 ToolHub 通用能力包 JSON 文件")
        })
        .await
        .map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "macos"))]
        Err("当前平台暂不支持能力包选择器".into())
    }
}
pub async fn save_bundle_file(
    window: tauri::WebviewWindow,
    name: String,
) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map_err(|e| e.to_string())?.0 as usize;
        tauri::async_runtime::spawn_blocking(move || {
            use windows::{
                core::{w, HSTRING},
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
            unsafe {
                CoInitializeEx(None, COINIT_APARTMENTTHREADED)
                    .ok()
                    .map_err(|e| e.to_string())?;
                let _apartment = Apartment;
                let dialog: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                        .map_err(|e| e.to_string())?;
                dialog
                    .SetOptions(
                        FOS_FORCEFILESYSTEM
                            | FOS_PATHMUSTEXIST
                            | FOS_NOCHANGEDIR
                            | FOS_OVERWRITEPROMPT,
                    )
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetTitle(w!("导出 ToolHub 通用能力包"))
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetFileName(&HSTRING::from(name))
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetDefaultExtension(w!("json"))
                    .map_err(|e| e.to_string())?;
                dialog
                    .SetFileTypes(&[COMDLG_FILTERSPEC {
                        pszName: w!("ToolHub 能力包"),
                        pszSpec: w!("*.toolhub-skill.json"),
                    }])
                    .map_err(|e| e.to_string())?;
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
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        #[cfg(target_os = "macos")]
        return tauri::async_runtime::spawn_blocking(move || crate::macos::choose("save", &name))
            .await
            .map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "macos"))]
        {
            let _ = name;
            Err("当前平台暂不支持能力包导出选择器".into())
        }
    }
}
