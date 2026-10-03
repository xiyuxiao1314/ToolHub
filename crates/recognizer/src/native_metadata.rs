//! Bounded, read-only native format measurements. A format is never identity evidence.
use std::{collections::BTreeMap, io::Read, path::Path};
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct NativeMetadata {
    pub format: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
    pub product: Option<String>,
    pub publisher: Option<String>,
    pub original_filename: Option<String>,
}
fn u16le(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}
fn arch(machine: u32) -> Option<String> {
    Some(
        match machine {
            0x14c | 3 => "x86",
            0x8664 | 62 => "x86_64",
            0xaa64 | 183 => "aarch64",
            0x1c4 | 40 => "arm",
            _ => return None,
        }
        .into(),
    )
}
pub fn read(path: &Path) -> NativeMetadata {
    let Ok(file) = std::fs::File::open(path) else {
        return NativeMetadata::default();
    };
    let mut bytes = vec![];
    // Full read keeps version resources reachable on large tools (node.exe is ~90MB).
    if std::io::Read::take(file, 96 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return NativeMetadata::default();
    }
    let mut m = measure(&bytes);
    if m.version.is_none() {
        m.version = scan_version_fallback(&bytes);
    }
    m
}
/// Some PE files store version strings outside the parser's walk; scan UTF-16 keys.
fn scan_version_fallback(b: &[u8]) -> Option<String> {
    for key in [
        "FileVersion",
        "ProductVersion",
    ] {
        let wide: Vec<u8> = key.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        if let Some(idx) = find_subslice(b, &wide) {
            let start = idx + wide.len();
            let chunk = b.get(start..start.saturating_add(80))?;
            let mut i = 0;
            while i + 1 < chunk.len() && chunk[i] == 0 && chunk[i + 1] == 0 {
                i += 2;
            }
            let mut units = vec![];
            let mut j = i;
            while j + 1 < chunk.len() {
                let u = u16::from_le_bytes([chunk[j], chunk[j + 1]]);
                if u == 0 {
                    break;
                }
                units.push(u);
                j += 2;
            }
            let text = String::from_utf16_lossy(&units);
            if !text.is_empty() {
                return Some(normalize_pe_version(&text));
            }
        }
    }
    None
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Keep a comparable numeric core from strings like `2.47.1.windows.1`.
fn normalize_pe_version(raw: &str) -> String {
    let trimmed = raw.trim();
    if toolhub_core::VersionConstraint::parse(&format!("={trimmed}")).is_ok() {
        return trimmed.to_string();
    }
    // Leading `1.98.1 (build)` or `2.47.1.windows.1` -> `1.98.1` / `2.47.1`.
    let mut buf = String::new();
    for c in trimmed.chars() {
        if c.is_ascii_digit() || (c == '.' && !buf.is_empty() && !buf.ends_with('.')) {
            buf.push(c);
        } else {
            break;
        }
    }
    let parts: Vec<&str> = buf
        .trim_end_matches('.')
        .split('.')
        .filter(|s| !s.is_empty())
        .take(3)
        .collect();
    if parts.is_empty() {
        trimmed.to_string()
    } else {
        parts.join(".")
    }
}

pub fn measure(b: &[u8]) -> NativeMetadata {
    let mut m = NativeMetadata::default();
    if b.starts_with(b"MZ") {
        let Some(pe) = u32le(b, 0x3c).map(|n| n as usize) else {
            return m;
        };
        if b.get(pe..pe + 4) != Some(b"PE\0\0".as_slice()) {
            return m;
        }
        m.format = Some("pe".into());
        m.arch = u16le(b, pe + 4).and_then(|n| arch(n as u32));
        if let Some(strings) = pe_version(b, pe) {
            m.product = strings.get("ProductName").cloned();
            m.publisher = strings.get("CompanyName").cloned();
            m.original_filename = strings.get("OriginalFilename").cloned();
            m.version = strings
                .get("FileVersion")
                .or_else(|| strings.get("ProductVersion"))
                .cloned()
                .map(|v| normalize_pe_version(&v));
        }
        if m.version.is_none() {
            m.version = scan_version_fallback(b);
        }
    } else if b.starts_with(b"\x7fELF")
        && b.len() >= 20
        && [1, 2].contains(&b[4])
        && [1, 2].contains(&b[5])
    {
        m.format = Some("elf".into());
        let machine = if b[5] == 1 {
            u16::from_le_bytes([b[18], b[19]])
        } else {
            u16::from_be_bytes([b[18], b[19]])
        };
        m.arch = arch(machine as u32);
    } else if b.len() >= 8 {
        let magic = u32::from_be_bytes(b[..4].try_into().unwrap());
        if [0xfeedface, 0xfeedfacf, 0xcefaedfe, 0xcffaedfe].contains(&magic) {
            let cpu = if [0xcefaedfe, 0xcffaedfe].contains(&magic) {
                u32::from_le_bytes(b[4..8].try_into().unwrap())
            } else {
                u32::from_be_bytes(b[4..8].try_into().unwrap())
            };
            m.format = Some("mach_o".into());
            m.arch = match cpu {
                7 => Some("x86".into()),
                0x01000007 => Some("x86_64".into()),
                12 => Some("arm".into()),
                0x0100000c => Some("aarch64".into()),
                _ => None,
            };
        } else if [0xcafebabe, 0xbebafeca, 0xcafebabf, 0xbfbafeca].contains(&magic) {
            m.format = Some("mach_o_fat".into());
            m.arch = Some("universal".into());
        }
    }
    m
}
fn pe_version(b: &[u8], pe: usize) -> Option<BTreeMap<String, String>> {
    let count = u16le(b, pe + 6)? as usize;
    let opt = pe + 24;
    let size = u16le(b, pe + 20)? as usize;
    let directory = match u16le(b, opt)? {
        0x10b => opt + 96,
        0x20b => opt + 112,
        _ => return None,
    };
    let rva = u32le(b, directory + 16)?;
    let sections = opt + size;
    let map = |rva: u32| -> Option<usize> {
        for i in 0..count.min(96) {
            let s = sections + i * 40;
            let va = u32le(b, s + 12)?;
            let rawsize = u32le(b, s + 16)?;
            let raw = u32le(b, s + 20)?;
            if rva >= va && rva - va < rawsize {
                return Some(raw.checked_add(rva - va)? as usize);
            }
        }
        None
    };
    let base = map(rva)?;
    let entries = u16le(b, base + 12)? as usize + u16le(b, base + 14)? as usize;
    for i in 0..entries.min(512) {
        let e = base + 16 + i * 8;
        if u32le(b, e)? != 16 {
            continue;
        }
        let mut offset = u32le(b, e + 4)?;
        for _ in 0..3 {
            if offset & 0x80000000 == 0 {
                break;
            }
            let node = base + (offset & 0x7fffffff) as usize;
            if u16le(b, node + 12)? as usize + u16le(b, node + 14)? as usize == 0 {
                return None;
            }
            offset = u32le(b, node + 20)?;
        }
        if offset & 0x80000000 != 0 {
            return None;
        }
        let data = base + offset as usize;
        let start = map(u32le(b, data)?)?;
        let len = u32le(b, data + 4)? as usize;
        let blob = b.get(start..start + len.min(1024 * 1024))?;
        let mut strings = BTreeMap::new();
        parse_block(blob, 0, blob.len(), 0, &mut strings);
        return Some(strings);
    }
    None
}
fn parse_block(b: &[u8], at: usize, end: usize, depth: usize, out: &mut BTreeMap<String, String>) {
    if depth > 8 || at + 6 > end {
        return;
    }
    let Some(len) = u16le(b, at).map(|x| x as usize) else {
        return;
    };
    if len < 6 || at + len > end {
        return;
    }
    let value_len = u16le(b, at + 2).unwrap_or(0) as usize;
    let kind = u16le(b, at + 4).unwrap_or(0);
    let mut pos = at + 6;
    let mut key = vec![];
    while pos + 2 <= at + len {
        let c = u16le(b, pos).unwrap();
        pos += 2;
        if c == 0 {
            break;
        }
        key.push(c)
    }
    pos = (pos + 3) & !3;
    let key = String::from_utf16_lossy(&key);
    let size = if kind == 1 {
        value_len.saturating_mul(2)
    } else {
        value_len
    };
    if let Some(value) = b
        .get(pos..pos.saturating_add(size))
        .filter(|_| pos + size <= at + len)
    {
        if kind == 1 && value_len > 0 {
            let value: Vec<_> = value
                .as_chunks::<2>()
                .0
                .iter()
                .map(|x| u16::from_le_bytes(*x))
                .take_while(|x| *x != 0)
                .collect();
            out.insert(key, String::from_utf16_lossy(&value));
        }
    }
    pos = (pos.saturating_add(size) + 3) & !3;
    while pos + 6 <= at + len {
        let child = u16le(b, pos).unwrap_or(0) as usize;
        if child < 6 {
            break;
        }
        parse_block(b, pos, at + len, depth + 1, out);
        pos = (pos.saturating_add(child) + 3) & !3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn elf_and_macho_architecture_are_observed_cross_platform() {
        let mut elf = vec![0u8; 64];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[18..20].copy_from_slice(&183u16.to_le_bytes());
        let metadata = measure(&elf);
        assert_eq!(metadata.format.as_deref(), Some("elf"));
        assert_eq!(metadata.arch.as_deref(), Some("aarch64"));
        assert!(metadata.version.is_none());
        let mut macho = vec![0u8; 32];
        macho[..4].copy_from_slice(&0xfeedfacfu32.to_le_bytes());
        macho[4..8].copy_from_slice(&0x01000007u32.to_le_bytes());
        let metadata = measure(&macho);
        assert_eq!(metadata.format.as_deref(), Some("mach_o"));
        assert_eq!(metadata.arch.as_deref(), Some("x86_64"));
    }
}
