//! Read icon resources without running the tool or invoking shell extension handlers.
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct IconPixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[tauri::command]
pub async fn tool_file_icon(path: String) -> Result<Option<IconPixels>, String> {
    tauri::async_runtime::spawn_blocking(move || extract(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())
}

fn extract(path: &Path) -> Option<IconPixels> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("ico"))
        || !path.is_file()
    {
        return None;
    }
    #[cfg(windows)]
    {
        windows_icon::extract(path).ok().or_else(|| {
            // javac.exe often has no icon; use its own JDK's Java launcher icon.
            if path
                .file_stem()
                .is_some_and(|stem| stem.eq_ignore_ascii_case("javac"))
            {
                let java = path.parent()?.join("java.exe");
                if java.is_file() {
                    return windows_icon::extract(&java).ok();
                }
            }
            None
        })
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
mod windows_icon {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::Shell::SHDefExtractIconW;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON};

    const SIDE: usize = 64;
    struct Icon(HICON);
    impl Drop for Icon {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }
    struct Surface {
        dc: HDC,
        bitmap: HBITMAP,
        old: HGDIOBJ,
        bits: *mut u8,
    }
    impl Drop for Surface {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.dc, self.old);
                let _ = DeleteObject(self.bitmap.into());
                let _ = DeleteDC(self.dc);
            }
        }
    }
    impl Surface {
        fn new() -> Result<Self, String> {
            let dc = unsafe { CreateCompatibleDC(None) };
            if dc.is_invalid() {
                return Err("icon DC unavailable".into());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: SIDE as i32,
                    biHeight: -(SIDE as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = match unsafe {
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            } {
                Ok(bitmap) => bitmap,
                Err(error) => {
                    unsafe {
                        let _ = DeleteDC(dc);
                    }
                    return Err(error.to_string());
                }
            };
            let old = unsafe { SelectObject(dc, bitmap.into()) };
            if old.is_invalid() || bits.is_null() {
                unsafe {
                    let _ = DeleteObject(bitmap.into());
                    let _ = DeleteDC(dc);
                }
                return Err("icon bitmap unavailable".into());
            }
            Ok(Self {
                dc,
                bitmap,
                old,
                bits: bits.cast(),
            })
        }
        fn render(&mut self, icon: HICON, background: u8) -> Result<Vec<u8>, String> {
            let pixels = unsafe { std::slice::from_raw_parts_mut(self.bits, SIDE * SIDE * 4) };
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.copy_from_slice(&[background, background, background, 255]);
            }
            unsafe {
                DrawIconEx(
                    self.dc,
                    0,
                    0,
                    icon,
                    SIDE as i32,
                    SIDE as i32,
                    0,
                    None,
                    DI_NORMAL,
                )
                .map_err(|e| e.to_string())?;
                GdiFlush().ok().map_err(|e| e.to_string())?;
            }
            Ok(pixels.to_vec())
        }
    }

    pub(super) fn extract(path: &Path) -> Result<IconPixels, String> {
        let wide: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut handle = HICON::default();
        let result = unsafe {
            SHDefExtractIconW(
                PCWSTR(wide.as_ptr()),
                0,
                0,
                Some(&mut handle),
                None,
                SIDE as u32,
            )
        };
        if handle.is_invalid() {
            return Err("no embedded icon".into());
        }
        let icon = Icon(handle);
        result.ok().map_err(|e| e.to_string())?;
        let mut surface = Surface::new()?;
        // Two matte renders reconstruct transparency for both legacy mask icons and alpha icons.
        let black = surface.render(icon.0, 0)?;
        let white = surface.render(icon.0, 255)?;
        let rgba = restore_alpha(&black, &white);
        if !rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0) {
            return Err("empty icon".into());
        }
        Ok(IconPixels {
            width: SIDE as u32,
            height: SIDE as u32,
            rgba,
        })
    }

    fn restore_alpha(black: &[u8], white: &[u8]) -> Vec<u8> {
        black
            .as_chunks::<4>()
            .0
            .iter()
            .zip(white.as_chunks::<4>().0.iter())
            .flat_map(|(b, w)| {
                let transparency = (0..3)
                    .map(|i| w[i].saturating_sub(b[i]))
                    .max()
                    .unwrap_or(255);
                let alpha = 255 - transparency;
                let channel = |i: usize| {
                    if alpha == 0 {
                        0
                    } else {
                        ((u32::from(b[i]) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255)
                            as u8
                    }
                };
                [channel(2), channel(1), channel(0), alpha]
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn restores_alpha_and_converts_bgra_to_rgba() {
            assert_eq!(
                restore_alpha(&[0, 0, 0, 255], &[255, 255, 255, 255]),
                vec![0, 0, 0, 0]
            );
            assert_eq!(
                restore_alpha(&[30, 60, 120, 255], &[30, 60, 120, 255]),
                vec![120, 60, 30, 255]
            );
            assert_eq!(
                restore_alpha(&[0, 0, 128, 255], &[127, 127, 255, 255]),
                vec![255, 0, 0, 128]
            );
        }
        #[test]
        fn extracts_real_application_icon_with_transparency() {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
            let icon = super::super::extract(&path).expect("project icon");
            assert_eq!(icon.rgba.len(), 64 * 64 * 4);
            assert!(icon.rgba.as_chunks::<4>().0.iter().any(|p| p[3] == 0));
            assert!(icon.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_or_non_icon_files_fall_back_without_execution() {
        assert!(extract(Path::new("relative.exe")).is_none());
        assert!(extract(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("Cargo.toml")
                .as_path()
        )
        .is_none());
        assert!(extract(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("missing.exe")
                .as_path()
        )
        .is_none());
    }
}
