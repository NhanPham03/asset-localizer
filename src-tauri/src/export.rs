use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(serde::Deserialize)]
pub struct ExportEntry {
    pub path: String,
    pub relative_path: String,
}

pub fn create_images_zip(entries: &[ExportEntry], output_path: &Path) -> Result<(), String> {
    let file = File::create(output_path).map_err(|err| {
        format!(
            "Failed to create zip at \"{}\": {err}",
            output_path.display()
        )
    })?;

    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in entries {
        let source = Path::new(&entry.path);
        if !source.is_file() {
            return Err(format!("File not found: {}", entry.path));
        }

        let zip_name = entry.relative_path.replace('\\', "/");
        archive
            .start_file(zip_name, options)
            .map_err(|err| format!("Failed to add \"{}\" to zip: {err}", entry.relative_path))?;

        let mut source_file = File::open(source)
            .map_err(|err| format!("Failed to open \"{}\": {err}", entry.path))?;

        let mut buffer = Vec::new();
        source_file
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Failed to read \"{}\": {err}", entry.path))?;

        archive
            .write_all(&buffer)
            .map_err(|err| format!("Failed to write \"{}\" to zip: {err}", entry.relative_path))?;
    }

    archive
        .finish()
        .map_err(|err| format!("Failed to finalize zip: {err}"))?;

    Ok(())
}
