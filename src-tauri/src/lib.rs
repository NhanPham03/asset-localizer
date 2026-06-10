mod atlas;
mod export;
mod ocr;
mod scanner;

use atlas::{AtlasIndex, AtlasInfo, AtlasRegion};
use export::{create_images_zip, ExportEntry};
use ocr::{OcrImageResult, OcrLogEntry, OcrSettings, RegionOcrResult};
use scanner::scan_directory;
use std::sync::Mutex;
use tauri::Emitter;

static OCR_LOGS: Mutex<Vec<OcrLogEntry>> = Mutex::new(Vec::new());

fn push_ocr_log(entry: OcrLogEntry) {
    if let Ok(mut logs) = OCR_LOGS.lock() {
        logs.push(entry);
    }
}

fn clear_ocr_logs() {
    if let Ok(mut logs) = OCR_LOGS.lock() {
        logs.clear();
    }
}

#[derive(serde::Serialize)]
struct ImageScanResult {
    path: String,
    relative_path: String,
    atlas: Option<AtlasInfo>,
}

fn map_image_results(
    root: &std::path::Path,
    paths: Vec<std::path::PathBuf>,
    atlas_index: &AtlasIndex,
) -> Vec<ImageScanResult> {
    paths
        .into_iter()
        .map(|path| {
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let atlas = atlas::atlas_for_image_with_index(&path, atlas_index);

            ImageScanResult {
                path: path.to_string_lossy().into_owned(),
                relative_path,
                atlas,
            }
        })
        .collect()
}

/// Scan a project directory and return all image file paths with paired atlas configs.
#[tauri::command]
fn scan_project_images(root_dir: String) -> Result<Vec<ImageScanResult>, String> {
    let root = std::path::Path::new(&root_dir);
    let images = scan_directory(root)?;
    let atlas_index = atlas::build_atlas_index(root)?;
    Ok(map_image_results(root, images, &atlas_index))
}

/// Return buffered OCR log entries.
#[tauri::command]
fn get_ocr_logs() -> Vec<OcrLogEntry> {
    OCR_LOGS.lock().map(|logs| logs.clone()).unwrap_or_default()
}

/// Clear buffered OCR log entries.
#[tauri::command]
fn clear_ocr_logs_cmd(app: tauri::AppHandle) -> Result<(), String> {
    clear_ocr_logs();
    let _ = app.emit("ocr-log-clear", ());
    Ok(())
}

/// Run OCR on a list of image paths and return text detection results.
#[tauri::command]
async fn ocr_scan_images(
    app: tauri::AppHandle,
    paths: Vec<String>,
    settings: OcrSettings,
) -> Result<Vec<OcrImageResult>, String> {
    let engine = ocr::OcrEnginePaths::new(&app)?;

    clear_ocr_logs();
    let _ = app.emit("ocr-log-clear", ());

    tokio::task::spawn_blocking(move || {
        engine.scan_images(
            &paths,
            settings,
            |progress| {
                let _ = app.emit("ocr-progress", progress);
            },
            |log| {
                push_ocr_log(log.clone());
                let _ = app.emit("ocr-log", log);
            },
            |result| {
                let _ = app.emit("ocr-result", result);
            },
        )
    })
    .await
    .map_err(|err| format!("OCR task failed: {err}"))?
}

/// Run OCR on atlas region crops for a single image.
#[tauri::command]
async fn ocr_atlas_regions(
    app: tauri::AppHandle,
    image_path: String,
    regions: Vec<AtlasRegion>,
    config_type: String,
    settings: OcrSettings,
) -> Result<Vec<RegionOcrResult>, String> {
    let engine = ocr::OcrEnginePaths::new(&app)?;

    tokio::task::spawn_blocking(move || {
        engine.ocr_regions(
            std::path::Path::new(&image_path),
            &regions,
            &config_type,
            settings,
        )
    })
    .await
    .map_err(|err| format!("Region OCR task failed: {err}"))?
}

/// Export image files into a zip archive, preserving relative paths.
#[tauri::command]
fn export_images_zip(entries: Vec<ExportEntry>, output_path: String) -> Result<(), String> {
    create_images_zip(&entries, std::path::Path::new(&output_path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_project_images,
            get_ocr_logs,
            clear_ocr_logs_cmd,
            ocr_scan_images,
            ocr_atlas_regions,
            export_images_zip
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
