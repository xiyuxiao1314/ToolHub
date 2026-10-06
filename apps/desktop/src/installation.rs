//! Read-only resources in portable packages and standard macOS app bundles.
use std::path::{Path, PathBuf};

pub fn asset_root(executable: &Path) -> Result<PathBuf, String> {
    let directory = executable
        .parent()
        .ok_or("installation directory unavailable")?;
    if directory.file_name().is_some_and(|name| name == "MacOS") {
        if let Some(contents) = directory.parent() {
            if contents.file_name().is_some_and(|name| name == "Contents")
                && contents
                    .parent()
                    .and_then(Path::extension)
                    .is_some_and(|ext| ext == "app")
            {
                return Ok(contents.join("Resources"));
            }
        }
    }
    Ok(directory.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_skills_are_read_from_portable_and_native_app_resources() {
        let fixture =
            std::env::temp_dir().join(format!("toolhub-installed-assets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&fixture).unwrap();
        let portable = fixture.join("ToolHub portable");
        let app = fixture.join("ToolHub 示例.app/Contents");
        let layouts = [
            (portable.join("toolhub-desktop"), portable.clone()),
            (app.join("MacOS/toolhub-desktop"), app.join("Resources")),
        ];
        for (executable, resources) in layouts {
            std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
            std::fs::create_dir_all(resources.join("skills/market")).unwrap();
            std::fs::create_dir_all(resources.join("skills/library/pdf-text-ocr")).unwrap();
            std::fs::write(
                resources.join("skills/market/fixture.json"),
                b"owned market fixture",
            )
            .unwrap();
            std::fs::write(
                resources.join("skills/library/pdf-text-ocr/skill.json"),
                b"owned skill fixture",
            )
            .unwrap();
            let resolved = asset_root(&executable).unwrap();
            assert_eq!(resolved, resources);
            assert_eq!(
                std::fs::read(resolved.join("skills/market/fixture.json")).unwrap(),
                b"owned market fixture"
            );
            assert!(resolved
                .join("skills/library/pdf-text-ocr/skill.json")
                .is_file());
        }
        let resolved_fixture = std::fs::canonicalize(&fixture).unwrap();
        let temp_root = std::fs::canonicalize(std::env::temp_dir()).unwrap();
        assert_eq!(resolved_fixture.parent(), Some(temp_root.as_path()));
        assert!(resolved_fixture
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("toolhub-installed-assets-"));
        std::fs::remove_dir_all(resolved_fixture).unwrap();
    }
}
