use std::fs;
use std::path::Path;

const MODELS_DIR: &str = "ocr-models";

struct ModelFile {
    url: &'static str,
    filename: &'static str,
}

const MODEL_FILES: &[ModelFile] = &[
    ModelFile {
        url: "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.7.0/mnn/PP-OCRv5/det/ch_PP-OCRv5_mobile_det.mnn",
        filename: "PP-OCRv5_mobile_det.mnn",
    },
    ModelFile {
        url: "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.7.0/mnn/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile_infer.mnn",
        filename: "PP-OCRv5_mobile_rec.mnn",
    },
    ModelFile {
        url: "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.7.0/paddle/PP-OCRv5/rec/ch_PP-OCRv5_rec_mobile_infer/ppocrv5_dict.txt",
        filename: "ppocr_keys_v5.txt",
    },
    ModelFile {
        url: "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.7.0/mnn/PP-OCRv4/cls/ch_ppocr_mobile_v2.0_cls_infer.mnn",
        filename: "PP-LCNet_textline_ori.mnn",
    },
];

fn download_model_file(dest: &Path, model: &ModelFile) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|err| format!("Failed to create HTTP client: {err}"))?;

    let response = client
        .get(model.url)
        .send()
        .map_err(|err| format!("Failed to download {}: {err}", model.url))?
        .error_for_status()
        .map_err(|err| format!("Failed to download {}: {err}", model.url))?;

    let bytes = response
        .bytes()
        .map_err(|err| format!("Failed to read {}: {err}", model.url))?;

    fs::write(dest, bytes).map_err(|err| format!("Failed to write {}: {err}", dest.display()))?;

    Ok(())
}

fn ensure_ocr_models() {
    let models_dir = Path::new(MODELS_DIR);
    if let Err(err) = fs::create_dir_all(models_dir) {
        println!("cargo:warning=Failed to create {MODELS_DIR} directory: {err}");
        return;
    }

    for model in MODEL_FILES {
        let dest = models_dir.join(model.filename);
        if dest.is_file() {
            continue;
        }

        match download_model_file(&dest, model) {
            Ok(()) => println!("cargo:warning=Downloaded OCR model: {}", model.filename),
            Err(err) => println!("cargo:warning={err}"),
        }
    }

    println!("cargo:rerun-if-changed={MODELS_DIR}");
}

fn main() {
    ensure_ocr_models();
    tauri_build::build();
}
