use std::path::{Path, PathBuf};

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false)
}

/// Recursively scan a directory for image files.
pub fn scan_directory(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.is_dir() {
        return Err(format!("\"{}\" is not a directory", dir.display()));
    }

    let mut files = Vec::new();
    collect_images(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_images(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|err| format!("Failed to read directory \"{}\": {err}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|err| format!("Failed to read entry in \"{}\": {err}", dir.display()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_images(&path, files)?;
        } else if is_image_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}
