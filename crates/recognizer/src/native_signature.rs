//! Native Windows chain/catalog validation. Never launches the inspected executable.
use std::path::Path;
use toolhub_core::{Evidence, EvidenceSource, Provenance, TrustRecord};
pub const REASON: &str = "native_windows_system_signature_v1";

/// Binding is checked again against the executor's current hash before any approval.
pub fn binding_valid(record: &str, path: &str, sha256: &str) -> bool {
    let Ok(record) = serde_json::from_str::<TrustRecord>(record) else {
        return false;
    };
    if record.reason.as_deref() != Some(REASON) {
        return true;
    }
    record
        .evidence
        .iter()
        .filter(|e| e.source == EvidenceSource::CodeSignature && e.provenance == Provenance::Native)
        .filter_map(|e| {
            e.detail
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        })
        .any(|v| {
            v["sha256"] == sha256
                && v["canonical_path"]
                    .as_str()
                    .is_some_and(|p| key(p) == key(path))
        })
}
fn key(p: &str) -> String {
    p.trim_start_matches(r"\\?\")
        .replace('\\', "/")
        .to_lowercase()
}

/// Identity exception is intentionally confined to actual OS curl, not arbitrary System32 names.
pub fn system_curl(path: &Path) -> Option<Evidence> {
    #[cfg(windows)]
    {
        windows::system_curl(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}
#[cfg(windows)]
mod windows {
    use super::*;
    use ::windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{HANDLE, HWND},
            Security::{
                Cryptography::{Catalog::*, CertGetNameStringW, CERT_NAME_SIMPLE_DISPLAY_TYPE},
                WinTrust::*,
            },
            System::SystemInformation::GetSystemDirectoryW,
        },
    };
    use sha2::{Digest, Sha256};
    use std::{
        fs::OpenOptions,
        io::{Read, Seek, SeekFrom},
        mem::size_of,
        os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    };
    pub fn system_curl(path: &Path) -> Option<Evidence> {
        let mut buf = [0u16; 32768];
        let n = unsafe { GetSystemDirectoryW(Some(&mut buf)) } as usize;
        if n == 0 || n >= buf.len() {
            return None;
        }
        let expected =
            std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n])).join("curl.exe");
        let canonical = path.canonicalize().ok()?;
        if key(&canonical.to_string_lossy())
            != key(&expected.canonicalize().ok()?.to_string_lossy())
        {
            return None;
        }
        // Prevent replacement/writes through the entire signature+content measurement.
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&canonical)
            .ok()?;
        if file.metadata().ok()?.len() > 128 * 1024 * 1024 {
            return None;
        }
        let wide: Vec<u16> = canonical
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let handle = HANDLE(file.as_raw_handle());
        let mut info = WINTRUST_FILE_INFO {
            cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(wide.as_ptr()),
            hFile: handle,
            ..Default::default()
        };
        let signer = unsafe { verify(WTD_CHOICE_FILE, WINTRUST_DATA_0 { pFile: &mut info }) }
            .map(|s| (s, "embedded"))
            .or_else(|| unsafe { catalog(handle, &wide) }.map(|s| (s, "catalog")))?;
        if ![
            "Microsoft Windows",
            "Microsoft Corporation",
            "Microsoft Windows Publisher",
            "Microsoft 3rd Party Application Component",
        ]
        .iter()
        .any(|v| signer.0.eq_ignore_ascii_case(v))
        {
            return None;
        }
        file.seek(SeekFrom::Start(0)).ok()?;
        let mut hash = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = file.read(&mut chunk).ok()?;
            if n == 0 {
                break;
            }
            hash.update(&chunk[..n]);
        }
        let sha256 = format!("{:x}", hash.finalize());
        let mut e=Evidence::native(EvidenceSource::CodeSignature,1.0,format!("Windows {} signature verified for OS cURL; signer {}; offline Windows trust cache; not a malware verdict",signer.1,signer.0)).ok()?;
        e.detail=Some(serde_json::json!({"verifier":"WinVerifyTrust","signature_type":signer.1,"signer":signer.0,"canonical_path":canonical,"sha256":sha256,"network_used":false}).to_string());
        Some(e)
    }
    #[cfg(test)]
    #[test]
    fn real_windows_catalog_member_is_verified() {
        let p = Path::new(r"C:\Windows\System32\curl.exe");
        if !p.exists() {
            return;
        }
        let file = OpenOptions::new().read(true).share_mode(1).open(p).unwrap();
        let wide: Vec<u16> = p.to_string_lossy().encode_utf16().chain(Some(0)).collect();
        let subject = unsafe { catalog(HANDLE(file.as_raw_handle()), &wide) }
            .expect("actual OS curl catalog");
        assert!(subject.starts_with("Microsoft"));
    }
    unsafe fn verify(choice: WINTRUST_DATA_UNION_CHOICE, info: WINTRUST_DATA_0) -> Option<String> {
        let mut data = WINTRUST_DATA {
            cbStruct: size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: choice,
            Anonymous: info,
            dwStateAction: WTD_STATEACTION_VERIFY,
            dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL | WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
            ..Default::default()
        };
        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let result = unsafe {
            WinVerifyTrust(
                HWND::default(),
                &mut action,
                (&mut data as *mut WINTRUST_DATA).cast(),
            )
        };
        let signer = if result == 0 {
            unsafe { signer(data.hWVTStateData) }
        } else {
            None
        };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        let _ = unsafe {
            WinVerifyTrust(
                HWND::default(),
                &mut action,
                (&mut data as *mut WINTRUST_DATA).cast(),
            )
        };
        signer
    }
    unsafe fn signer(state: HANDLE) -> Option<String> {
        let provider = unsafe { WTHelperProvDataFromStateData(state) };
        if provider.is_null() {
            return None;
        }
        let signer = unsafe { WTHelperGetProvSignerFromChain(provider, 0, false, 0) };
        if signer.is_null() {
            return None;
        }
        let cert = unsafe { WTHelperGetProvCertFromChain(signer, 0) };
        if cert.is_null() {
            return None;
        }
        let mut name = [0u16; 512];
        let n = unsafe {
            CertGetNameStringW(
                (*cert).pCert,
                CERT_NAME_SIMPLE_DISPLAY_TYPE,
                0,
                None,
                Some(&mut name),
            )
        } as usize;
        if n < 2 || n > name.len() {
            return None;
        }
        Some(String::from_utf16_lossy(&name[..n - 1]))
    }
    unsafe fn catalog(handle: HANDLE, path: &[u16]) -> Option<String> {
        struct Admin(isize);
        impl Drop for Admin {
            fn drop(&mut self) {
                let _ = unsafe { CryptCATAdminReleaseContext(self.0, 0) };
            }
        }
        struct Cat(isize, isize);
        impl Drop for Cat {
            fn drop(&mut self) {
                let _ = unsafe { CryptCATAdminReleaseCatalogContext(self.0, self.1, 0) };
            }
        }
        let mut admin = 0;
        unsafe { CryptCATAdminAcquireContext2(&mut admin, None, w!("SHA256"), None, None) }.ok()?;
        let admin = Admin(admin);
        let mut len = 0;
        let _ =
            unsafe { CryptCATAdminCalcHashFromFileHandle2(admin.0, handle, &mut len, None, None) };
        if len == 0 || len > 128 {
            return None;
        }
        let mut hash = vec![0u8; len as usize];
        unsafe {
            CryptCATAdminCalcHashFromFileHandle2(
                admin.0,
                handle,
                &mut len,
                Some(hash.as_mut_ptr()),
                None,
            )
        }
        .ok()?;
        hash.truncate(len as usize);
        let found = unsafe { CryptCATAdminEnumCatalogFromHash(admin.0, &hash, None, None) };
        if found == 0 {
            return None;
        }
        let cat = Cat(admin.0, found);
        let mut info = CATALOG_INFO {
            cbStruct: size_of::<CATALOG_INFO>() as u32,
            ..Default::default()
        };
        unsafe { CryptCATCatalogInfoFromContext(cat.1, &mut info, 0) }.ok()?;
        let tag: Vec<u16> = hex::encode_upper(&hash)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut trust = WINTRUST_CATALOG_INFO {
            cbStruct: size_of::<WINTRUST_CATALOG_INFO>() as u32,
            pcwszCatalogFilePath: PCWSTR(info.wszCatalogFile.as_ptr()),
            pcwszMemberTag: PCWSTR(tag.as_ptr()),
            pcwszMemberFilePath: PCWSTR(path.as_ptr()),
            hMemberFile: handle,
            pbCalculatedFileHash: hash.as_mut_ptr(),
            cbCalculatedFileHash: hash.len() as u32,
            hCatAdmin: admin.0,
            ..Default::default()
        };
        unsafe {
            verify(
                WTD_CHOICE_CATALOG,
                WINTRUST_DATA_0 {
                    pCatalog: &mut trust,
                },
            )
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signature_binding_rejects_changed_content_and_path() {
        let mut t = TrustRecord::known(vec![]);
        t.reason = Some(REASON.into());
        assert!(!binding_valid(
            &serde_json::to_string(&t).unwrap(),
            "C:/Windows/System32/curl.exe",
            "abc"
        ));
        let mut e = Evidence::native(EvidenceSource::CodeSignature, 1.0, "fixture").unwrap();
        e.detail = Some(
            serde_json::json!({"canonical_path":"C:/Windows/System32/curl.exe","sha256":"abc"})
                .to_string(),
        );
        t.evidence.push(e);
        let raw = serde_json::to_string(&t).unwrap();
        assert!(binding_valid(
            &raw,
            r"\\?\C:\Windows\System32\curl.exe",
            "abc"
        ));
        assert!(!binding_valid(
            &raw,
            "C:/Windows/System32/curl.exe",
            "changed"
        ));
        assert!(!binding_valid(&raw, "D:/curl.exe", "abc"));
    }
    #[cfg(windows)]
    #[test]
    fn real_catalog_and_copied_or_fake_curl_are_distinct() {
        let original = Path::new(r"C:\Windows\System32\curl.exe");
        if original.exists() {
            let e = system_curl(original).expect("native OS cURL signature");
            assert!(e.summary.contains("Microsoft"));
            let d = tempfile::tempdir().unwrap();
            let copy = d.path().join("curl.exe");
            std::fs::copy(original, &copy).unwrap();
            assert!(system_curl(&copy).is_none());
            std::fs::write(&copy, b"MZ fake curl").unwrap();
            assert!(system_curl(&copy).is_none());
        }
    }
}
