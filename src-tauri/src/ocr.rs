use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::atlas::AtlasRegion;
use ocr_rs::{Backend, OcrEngine, OcrEngineConfig, OriOptions};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri::Manager;

const MILESTONE_INTERVAL: usize = 50;

#[derive(Clone, Serialize)]
pub struct OcrImageResult {
    pub path: String,
    pub has_text: bool,
    pub detected_text: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OcrLogEntry {
    Milestone {
        current: usize,
        total: usize,
        text_count: usize,
    },
    Text {
        index: usize,
        total: usize,
        path: String,
        detected_text: String,
    },
}

#[derive(Clone, Serialize)]
pub struct RegionOcrResult {
    pub name: String,
    pub has_text: bool,
    pub detected_text: String,
}

#[derive(Clone, Serialize)]
pub struct OcrProgress {
    pub current: usize,
    pub total: usize,
    pub path: String,
}

#[derive(Clone, Deserialize)]
pub struct OcrSettings {
    pub cpu_cores: usize,
    pub use_gpu: bool,
}

impl Default for OcrSettings {
    fn default() -> Self {
        Self {
            cpu_cores: std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(4)
                .max(1),
            use_gpu: false,
        }
    }
}

struct OcrLogState {
    completed: usize,
    text_count: usize,
}

pub struct OcrEnginePaths {
    det_model: PathBuf,
    rec_model: PathBuf,
    charset: PathBuf,
    ori_model: Option<PathBuf>,
}

pub fn has_meaningful_text(text: &str) -> bool {
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    if compact.is_empty() {
        return false;
    }

    compact.chars().any(|ch| {
        ch.is_alphanumeric()
            || ch.is_alphabetic()
            || matches!(
                ch,
                '\u{4E00}'..='\u{9FFF}'
                    | '\u{3400}'..='\u{4DBF}'
                    | '\u{3000}'..='\u{303F}'
                    | '\u{FF00}'..='\u{FFEF}'
            )
    })
}

fn has_model_files(dir: &Path) -> bool {
    dir.is_dir()
        && dir.join("PP-OCRv5_mobile_det.mnn").is_file()
        && dir.join("PP-OCRv5_mobile_rec.mnn").is_file()
        && dir.join("ppocr_keys_v5.txt").is_file()
}

fn resolve_ori_model(dir: &Path) -> Option<PathBuf> {
    let ori = dir.join("PP-LCNet_textline_ori.mnn");
    if ori.is_file() {
        Some(ori)
    } else {
        None
    }
}

fn resolve_models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("ocr-models");
        if has_model_files(&bundled) {
            return Ok(bundled);
        }
    }

    let dev_models = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ocr-models");
    if has_model_files(&dev_models) {
        return Ok(dev_models);
    }

    Err(
        "OCR model files not found. Rebuild the app so models are downloaded, \
         or place PP-OCRv5_mobile_det.mnn, PP-OCRv5_mobile_rec.mnn, and \
         ppocr_keys_v5.txt in src-tauri/ocr-models/."
            .to_string(),
    )
}

fn should_emit_milestone(current: usize, total: usize) -> bool {
    current == total || (current % MILESTONE_INTERVAL == 0 && current > 0)
}

fn emit_scan_logs(
    log_state: &Mutex<OcrLogState>,
    total: usize,
    result: &OcrImageResult,
    on_log: &impl Fn(OcrLogEntry),
) {
    let (current, text_count, emit_text) = {
        let mut state = log_state.lock().unwrap();
        state.completed += 1;
        if result.has_text {
            state.text_count += 1;
        }
        (state.completed, state.text_count, result.has_text)
    };

    if emit_text {
        on_log(OcrLogEntry::Text {
            index: current,
            total,
            path: result.path.clone(),
            detected_text: result.detected_text.clone(),
        });
    }

    if should_emit_milestone(current, total) {
        on_log(OcrLogEntry::Milestone {
            current,
            total,
            text_count,
        });
    }
}

fn build_engine_config(settings: &OcrSettings) -> OcrEngineConfig {
    let base = if settings.use_gpu {
        gpu_engine_config()
    } else {
        OcrEngineConfig::default()
    };

    base.with_threads(1)
        .with_ori_options(OriOptions::textline())
        .with_ori_min_confidence(0.3)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn gpu_engine_config() -> OcrEngineConfig {
    OcrEngineConfig {
        backend: Backend::Metal,
        ..Default::default()
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn gpu_engine_config() -> OcrEngineConfig {
    OcrEngineConfig {
        backend: Backend::OpenCL,
        ..Default::default()
    }
}

fn create_engine(
    det_model: &Path,
    rec_model: &Path,
    charset: &Path,
    ori_model: Option<&Path>,
    settings: &OcrSettings,
) -> Result<OcrEngine, String> {
    let det = det_model
        .to_str()
        .ok_or_else(|| "Invalid detection model path".to_string())?;
    let rec = rec_model
        .to_str()
        .ok_or_else(|| "Invalid recognition model path".to_string())?;
    let charset_str = charset
        .to_str()
        .ok_or_else(|| "Invalid charset path".to_string())?;
    let config = Some(build_engine_config(settings));

    let engine = match ori_model {
        Some(ori) => {
            let ori_str = ori
                .to_str()
                .ok_or_else(|| "Invalid orientation model path".to_string())?;
            OcrEngine::new_with_ori(det, rec, charset_str, ori_str, config)
        }
        None => OcrEngine::new(det, rec, charset_str, config),
    };

    engine.map_err(|err| format!("Failed to create OCR engine: {err}"))
}

fn scan_image_with_engine(engine: &OcrEngine, path: &Path) -> Result<OcrImageResult, String> {
    let image = image::open(path)
        .map_err(|err| format!("Failed to decode \"{}\": {err}", path.display()))?;

    let results = engine
        .recognize(&image)
        .map_err(|err| format!("OCR failed for \"{}\": {err}", path.display()))?;

    let detected_text = results
        .iter()
        .map(|result| result.text.as_str())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(OcrImageResult {
        path: path.to_string_lossy().into_owned(),
        has_text: has_meaningful_text(&detected_text),
        detected_text,
    })
}

fn crop_region_image(
    image: &image::DynamicImage,
    region: &AtlasRegion,
    config_type: &str,
) -> image::DynamicImage {
    let (iw, ih) = (image.width(), image.height());
    let w = region.width.max(1) as u32;
    let h = region.height.max(1) as u32;
    let x = region.x.max(0) as u32;
    let y = region.y.max(0) as u32;

    let x = x.min(iw.saturating_sub(1));
    let y = y.min(ih.saturating_sub(1));
    let w = w.min(iw.saturating_sub(x)).max(1);
    let h = h.min(ih.saturating_sub(y)).max(1);

    let mut crop = image.crop_imm(x, y, w, h);
    if region.rotated {
        crop = if config_type == "plist" {
            // Cocos packs 90° CW; undo with 90° CCW.
            crop.rotate270()
        } else {
            // LibGDX packs 90° CCW; undo with 90° CW.
            crop.rotate90()
        };
    }
    crop
}

fn recognize_crop(engine: &OcrEngine, crop: &image::DynamicImage) -> Result<String, String> {
    let results = engine
        .recognize(crop)
        .map_err(|err| format!("OCR failed on region crop: {err}"))?;

    Ok(results
        .iter()
        .map(|result| result.text.as_str())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

impl OcrEnginePaths {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let models_dir = resolve_models_dir(app)?;

        Ok(Self {
            det_model: models_dir.join("PP-OCRv5_mobile_det.mnn"),
            rec_model: models_dir.join("PP-OCRv5_mobile_rec.mnn"),
            charset: models_dir.join("ppocr_keys_v5.txt"),
            ori_model: resolve_ori_model(&models_dir),
        })
    }

    pub fn scan_images(
        &self,
        paths: &[String],
        settings: OcrSettings,
        on_progress: impl Fn(OcrProgress) + Send + Sync,
        on_log: impl Fn(OcrLogEntry) + Send + Sync,
        on_result: impl Fn(OcrImageResult) + Send + Sync,
    ) -> Result<Vec<OcrImageResult>, String> {
        let total = paths.len();
        if total == 0 {
            return Ok(Vec::new());
        }

        let max_cores = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(4)
            .max(1);

        let worker_count = settings.cpu_cores.clamp(1, max_cores).min(total);

        let next_job = AtomicUsize::new(0);
        let results = Mutex::new(vec![None; total]);
        let error = Mutex::new(None::<String>);
        let log_state = Mutex::new(OcrLogState {
            completed: 0,
            text_count: 0,
        });

        std::thread::scope(|scope| {
            for _ in 0..worker_count {
                scope.spawn(|| {
                    if error.lock().unwrap().is_some() {
                        return;
                    }

                    let engine = match create_engine(
                        &self.det_model,
                        &self.rec_model,
                        &self.charset,
                        self.ori_model.as_deref(),
                        &settings,
                    ) {
                        Ok(engine) => engine,
                        Err(err) => {
                            *error.lock().unwrap() = Some(err);
                            return;
                        }
                    };

                    loop {
                        if error.lock().unwrap().is_some() {
                            return;
                        }

                        let job = next_job.fetch_add(1, Ordering::Relaxed);
                        if job >= total {
                            break;
                        }

                        let path = &paths[job];
                        match scan_image_with_engine(&engine, Path::new(path)) {
                            Ok(result) => {
                                emit_scan_logs(&log_state, total, &result, &on_log);

                                on_result(result.clone());

                                let current = log_state.lock().unwrap().completed;
                                results.lock().unwrap()[job] = Some(result);
                                on_progress(OcrProgress {
                                    current,
                                    total,
                                    path: path.clone(),
                                });
                            }
                            Err(err) => {
                                *error.lock().unwrap() = Some(err);
                                return;
                            }
                        }
                    }
                });
            }
        });

        if let Some(err) = error.into_inner().unwrap() {
            return Err(err);
        }

        Ok(results
            .into_inner()
            .unwrap()
            .into_iter()
            .map(|result| result.expect("missing OCR result for completed job"))
            .collect())
    }

    pub fn ocr_regions(
        &self,
        image_path: &Path,
        regions: &[AtlasRegion],
        config_type: &str,
        settings: OcrSettings,
    ) -> Result<Vec<RegionOcrResult>, String> {
        if regions.is_empty() {
            return Ok(Vec::new());
        }

        let image = image::open(image_path)
            .map_err(|err| format!("Failed to decode \"{}\": {err}", image_path.display()))?;

        let engine = create_engine(
            &self.det_model,
            &self.rec_model,
            &self.charset,
            self.ori_model.as_deref(),
            &settings,
        )?;

        regions
            .iter()
            .map(|region| {
                let crop = crop_region_image(&image, region, config_type);
                let detected_text = recognize_crop(&engine, &crop)?;
                Ok(RegionOcrResult {
                    name: region.name.clone(),
                    has_text: has_meaningful_text(&detected_text),
                    detected_text,
                })
            })
            .collect()
    }
}
