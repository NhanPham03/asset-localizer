# Asset Localizer

Desktop app for scanning and localizing game project assets. Built with **Tauri 2**, **Svelte 5**, **TypeScript**, and **pnpm**.


## Prerequisites

### JavaScript / frontend

- [Node.js](https://nodejs.org/) **20.19+** or **22.12+** (required by Vite 8)
- [pnpm](https://pnpm.io/) — install with `npm install -g pnpm` or [other methods](https://pnpm.io/installation)

Frontend-only development (`pnpm dev`) needs only the above.

### Desktop app (Tauri)

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain via `rustup`)
- [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS:
  - **Windows** — [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **Desktop development with C++** workload, plus [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
  - **macOS** — Xcode or Xcode Command Line Tools (`xcode-select --install`)
  - **Linux** — `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, and related packages (see the Tauri guide for your distribution)

### OCR (Rust backend)

The `ocr-rs` crate uses `bindgen`, which requires **libclang** at compile time:

- Install [LLVM](https://releases.llvm.org/) and ensure `libclang` is on your `PATH`, **or**
- Set `LIBCLANG_PATH` to the directory containing `libclang.dll` (Windows) / `libclang.so` (Linux) / `libclang.dylib` (macOS)

On the first `cargo build`, PP-OCRv5 model files (~tens of MB) are downloaded automatically into `src-tauri/ocr-models/`. A network connection is required for that step. Prebuilt MNN libraries are also fetched automatically for supported platforms (Windows x64, macOS, Linux x64/aarch64).

## Getting Started

```bash
pnpm install
pnpm tauri dev
```

Frontend-only dev (no Tauri shell):

```bash
pnpm dev
```

Build for production:

```bash
pnpm tauri build
```

Remove generated artifacts before archiving or sharing the project (restored by `pnpm install` and the next build):

```bash
pnpm clean
```

## Project Structure

```
asset-localizer/
├── src/                 # Svelte frontend
├── src-tauri/           # Rust backend (Tauri)
│   └── src/scanner.rs   # Image directory scanner
└── package.json
```

## Features

- **Folder picking** — native folder dialog via Tauri
- **Image scanning** — recursively finds PNG, JPG, JPEG, GIF, BMP, WebP
- **List UI** — browse results with checkboxes, select all, hover preview
- **Path exclusion** — filter out files/folders by comma-separated terms
- **OCR text detection** — PP-OCRv5 Chinese recognition with text-line orientation correction; console shows milestones and text hits only
- **Export** — zip selected images (images only, or with paired `.atlas` / `.plist` configs), preserving folder structure

## Rust Commands

| Command | Description |
|---------|-------------|
| `scan_project_images` | Recursively find image files and pair atlas/plist configs |
| `ocr_scan_images` | Run OCR on images and detect text content |
| `ocr_atlas_regions` | Run OCR on atlas region crops for a single image |
| `get_ocr_logs` / `clear_ocr_logs_cmd` | Read or clear the OCR console buffer |
| `export_images_zip` | Write selected images to a zip archive |
