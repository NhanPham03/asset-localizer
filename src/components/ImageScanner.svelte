<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import type { OcrImageResult, OcrProgress, OcrSettings, ScannedImage } from "../types";
  import {
    applyOcrResults,
    buildExportEntries,
    defaultExcludeInput,
    defaultExportName,
    filterImages,
    parseExcludeTerms,
    searchImages,
    type ExportMode,
  } from "../lib/images";
  import { isTauriRuntime } from "../lib/tauri";
  import { loadSession, saveSession } from "../lib/sessionStore";
  import ImageList from "./ImageList.svelte";
  import OcrLogPanel from "./OcrLogPanel.svelte";

  interface Props {
    onOpenEditor?: (image: ScannedImage) => void;
    onImagesChange?: (images: ScannedImage[]) => void;
  }

  let { onOpenEditor, onImagesChange }: Props = $props();

  let rootDir = $state<string | null>(null);
  let projectName = $state("");
  let excludeInput = $state(defaultExcludeInput());
  let searchQuery = $state("");
  let images = $state<ScannedImage[]>([]);
  let selectedPaths = $state(new Set<string>());
  let scanning = $state(false);
  let ocrRunning = $state(false);
  let exporting = $state(false);
  let error = $state<string | null>(null);
  let status = $state<string | null>(null);
  let ocrProgress = $state<OcrProgress | null>(null);

  let maxCpuCores = $state(4);
  let ocrCpuCores = $state(4);
  let ocrUseGpu = $state(false);

  const excludeTerms = $derived(parseExcludeTerms(excludeInput));
  const visibleImages = $derived(filterImages(images, excludeTerms));
  const listedImages = $derived(searchImages(visibleImages, searchQuery));
  const excludedCount = $derived(images.length - visibleImages.length);
  const selectedVisibleCount = $derived(
    visibleImages.filter((image) => selectedPaths.has(image.path)).length,
  );
  const textAssetCount = $derived(
    visibleImages.filter((image) => image.has_text === true).length,
  );
  const noTextCount = $derived(
    visibleImages.filter((image) => image.has_text === false).length,
  );
  const ocrComplete = $derived(
    visibleImages.length > 0 &&
      visibleImages.every((image) => image.has_text !== undefined && image.has_text !== null),
  );
  const atlasCount = $derived(
    visibleImages.filter((image) => image.atlas && image.atlas.regions.length > 0).length,
  );
  const ocrSettings = $derived<OcrSettings>({
    cpu_cores: ocrCpuCores,
    use_gpu: ocrUseGpu,
  });

  function persistSession() {
    saveSession({
      rootDir,
      projectName,
      excludeInput,
      images,
      selectedPaths: [...selectedPaths],
    });
  }

  onMount(() => {
    const saved = loadSession();
    if (saved) {
      rootDir = saved.rootDir;
      projectName = saved.projectName;
      excludeInput = saved.excludeInput;
      images = saved.images;
      selectedPaths = new Set(saved.selectedPaths);
      onImagesChange?.(images);
    }

    const cores = navigator.hardwareConcurrency ?? 4;
    maxCpuCores = Math.max(1, cores);
    ocrCpuCores = maxCpuCores;

    const cleanups: Array<() => void> = [];

    listen<OcrProgress>("ocr-progress", (event) => {
      ocrProgress = event.payload;
    }).then((cleanup) => cleanups.push(cleanup));

    listen<OcrImageResult>("ocr-result", (event) => {
      images = applyOcrResults(images, [event.payload]);
      onImagesChange?.(images);
      persistSession();
    }).then((cleanup) => cleanups.push(cleanup));

    return () => {
      for (const cleanup of cleanups) cleanup();
    };
  });

  $effect(() => {
    rootDir;
    projectName;
    excludeInput;
    images;
    selectedPaths;
    if (images.length > 0 || rootDir) {
      persistSession();
    }
  });

  function resetOcrState() {
    images = images.map((image) => ({
      ...image,
      has_text: undefined,
      detected_text: undefined,
    }));
    ocrProgress = null;
  }

  async function runOcr(imageList: ScannedImage[]) {
    if (!isTauriRuntime()) {
      error = "OCR requires the Tauri desktop app. Run `pnpm tauri dev`.";
      return;
    }

    if (imageList.length === 0) return;

    ocrRunning = true;
    error = null;
    status = `Running OCR on ${imageList.length} image${imageList.length === 1 ? "" : "s"}...`;

    try {
      const results = await invoke<OcrImageResult[]>("ocr_scan_images", {
        paths: imageList.map((image) => image.path),
        settings: ocrSettings,
      });

      const textImages = results.filter((result) => result.has_text);
      selectedPaths = new Set(textImages.map((result) => result.path));

      status = `OCR complete: ${textImages.length} with text, ${results.length - textImages.length} without text`;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      status = null;
    } finally {
      ocrRunning = false;
      ocrProgress = null;
    }
  }

  async function scanFolder(folderPath: string) {
    scanning = true;
    error = null;
    status = "Scanning for images...";
    resetOcrState();

    try {
      const results = await invoke<ScannedImage[]>("scan_project_images", {
        rootDir: folderPath,
      });

      rootDir = folderPath;
      images = results;
      selectedPaths = new Set(results.map((image) => image.path));

      const folderName = folderPath.split(/[/\\]/).filter(Boolean).pop();
      if (folderName && !projectName.trim()) {
        projectName = folderName;
      }

      onImagesChange?.(images);
      persistSession();
      const pairedAtlas = results.filter(
        (image) => image.atlas && image.atlas.regions.length > 0,
      ).length;
      const atlasMsg =
        pairedAtlas > 0 ? ` ${pairedAtlas} with atlas config.` : "";
      status = `Found ${results.length} image${results.length === 1 ? "" : "s"}.${atlasMsg} Press "Scan with OCR" to detect text.`;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      status = null;
    } finally {
      scanning = false;
    }
  }

  async function pickFolder() {
    if (!isTauriRuntime()) {
      error =
        "Folder picking requires the Tauri desktop app. Run `pnpm tauri dev`.";
      return;
    }

    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select project folder",
    });

    if (typeof selected === "string") {
      await scanFolder(selected);
    }
  }

  async function rescan() {
    if (!rootDir) return;
    await scanFolder(rootDir);
  }

  async function detectText() {
    await runOcr(visibleImages);
  }

  function toggleImage(path: string) {
    const next = new Set(selectedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    selectedPaths = next;
  }

  function toggleAll(checked: boolean) {
    selectedPaths = checked
      ? new Set(visibleImages.map((image) => image.path))
      : new Set();
  }

  function selectWithText() {
    selectedPaths = new Set(
      visibleImages
        .filter((image) => image.has_text === true)
        .map((image) => image.path),
    );
  }

  function selectWithoutText() {
    selectedPaths = new Set(
      visibleImages
        .filter((image) => image.has_text === false)
        .map((image) => image.path),
    );
  }

  async function exportImages(mode: ExportMode) {
    if (!isTauriRuntime()) {
      error = "Export requires the Tauri desktop app. Run `pnpm tauri dev`.";
      return;
    }

    if (!rootDir) {
      error = "Pick a project folder before exporting.";
      return;
    }

    const source = visibleImages.filter((image) => selectedPaths.has(image.path));

    if (source.length === 0) {
      error = "Select at least one image to export.";
      return;
    }

    const entries = buildExportEntries(source, rootDir, mode);
    const configCount = entries.length - source.length;

    if (mode === "with-atlas" && configCount === 0) {
      error = "None of the selected images have a paired atlas or plist config.";
      return;
    }

    const outputPath = await save({
      title: mode === "with-atlas" ? "Save export with atlas configs" : "Save image export",
      defaultPath: defaultExportName(projectName, mode),
      filters: [{ name: "Zip archive", extensions: ["zip"] }],
    });

    if (!outputPath) return;

    exporting = true;
    error = null;
    const fileLabel =
      mode === "with-atlas"
        ? `${source.length} image${source.length === 1 ? "" : "s"} and ${configCount} config${configCount === 1 ? "" : "s"}`
        : `${source.length} image${source.length === 1 ? "" : "s"}`;
    status = `Exporting ${fileLabel}...`;

    try {
      await invoke("export_images_zip", {
        entries,
        outputPath,
      });

      status = `Exported ${fileLabel} to ${outputPath}`;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      status = null;
    } finally {
      exporting = false;
    }
  }

  const busy = $derived(scanning || ocrRunning || exporting);
</script>

<div class="workspace-layout">
  <aside class="sidebar sidebar-left">
    <h2 class="sidebar-title">Scan</h2>

    <div class="sidebar-actions">
      <button
        type="button"
        class="btn btn-primary btn-block"
        onclick={pickFolder}
        disabled={busy}
      >
        Pick Folder
      </button>
      <button
        type="button"
        class="btn btn-secondary btn-block"
        onclick={rescan}
        disabled={!rootDir || busy}
      >
        Rescan
      </button>
      <button
        type="button"
        class="btn btn-secondary btn-block"
        onclick={detectText}
        disabled={visibleImages.length === 0 || busy}
      >
        {ocrRunning ? "Scanning with OCR..." : "Scan with OCR"}
      </button>
    </div>

    <section class="ocr-settings">
      <h3 class="settings-title">Basic OCR settings</h3>

      <label class="field">
        CPU cores
        <div class="range-row">
          <input
            type="range"
            min="1"
            max={maxCpuCores}
            bind:value={ocrCpuCores}
            disabled={busy}
          />
          <span class="range-value">{ocrCpuCores}</span>
        </div>
        <span class="field-hint">Parallel workers (max {maxCpuCores})</span>
      </label>

      <label class="toggle-field">
        <input type="checkbox" bind:checked={ocrUseGpu} disabled={busy} />
        <span>Use GPU acceleration</span>
      </label>
      <span class="field-hint gpu-hint">
        {#if ocrUseGpu}
          OpenCL / Metal backend when available
        {:else}
          CPU inference (most compatible)
        {/if}
      </span>
    </section>

    <label class="field">
      Exclude paths containing
      <input
        type="text"
        bind:value={excludeInput}
        placeholder="library, temp, build"
      />
      <span class="field-hint">Defaults: library, temp, build</span>
    </label>

    {#if rootDir}
      <div class="folder-info">
        <span class="label">Folder</span>
        <span class="path" title={rootDir}>{rootDir}</span>
      </div>
    {/if}

    {#if error || status}
      <div class="message" class:error={!!error} class:info={!error}>
        {error ?? status}
        {#if ocrProgress}
          <div class="ocr-progress-detail">
            OCR {ocrProgress.current}/{ocrProgress.total}:
            {ocrProgress.path.split(/[/\\]/).pop()}
          </div>
        {/if}
      </div>
    {/if}
  </aside>

  <section class="column-center panel">
    <div class="panel-subheader">
      <h2>Images</h2>
      <div class="panel-badges">
        <span class="count-badge">{listedImages.length}{searchQuery.trim() ? ` / ${visibleImages.length}` : ""}</span>
        {#if ocrComplete}
          <span class="text-badge">{textAssetCount} text</span>
          <span class="no-text-badge">{noTextCount} no text</span>
        {/if}
        {#if excludedCount > 0}
          <span class="excluded-badge">{excludedCount} excluded</span>
        {/if}
        {#if atlasCount > 0}
          <span class="atlas-badge">{atlasCount} atlas</span>
        {/if}
      </div>
    </div>

    <label class="image-search">
      <span class="sr-only">Search images</span>
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Search by file name or path..."
        disabled={visibleImages.length === 0}
      />
    </label>

    <div class="panel-content">
      {#if scanning}
        <div class="empty-state">
          <p>Scanning folder...</p>
        </div>
      {:else}
        <ImageList
          images={listedImages}
          {selectedPaths}
          {ocrComplete}
          onToggle={toggleImage}
          onToggleAll={toggleAll}
          onSelectWithText={selectWithText}
          onSelectWithoutText={selectWithoutText}
          onOpenEditor={onOpenEditor}
        />
      {/if}
    </div>
  </section>

  <aside class="sidebar sidebar-right">
    <h2 class="sidebar-title">Export</h2>

    <label class="field">
      Project name
      <input
        type="text"
        bind:value={projectName}
        placeholder="Used for export file names"
      />
    </label>

    <div class="export-summary">
      <div class="summary-row">
        <span>Selected</span>
        <strong>{selectedVisibleCount}</strong>
      </div>
      {#if ocrComplete}
        <div class="summary-row">
          <span>Text assets</span>
          <strong>{textAssetCount}</strong>
        </div>
      {/if}
    </div>

    <div class="sidebar-actions">
      <button
        type="button"
        class="btn btn-export btn-block"
        onclick={() => exportImages("images")}
        disabled={busy || selectedVisibleCount === 0}
      >
        {exporting ? "Exporting..." : "Export Images Only"}
      </button>
      <button
        type="button"
        class="btn btn-export btn-block"
        onclick={() => exportImages("with-atlas")}
        disabled={busy || selectedVisibleCount === 0}
      >
        Export with Atlas Configs
      </button>
    </div>

    <OcrLogPanel {ocrRunning} />
  </aside>
</div>

<style>
  .workspace-layout {
    display: grid;
    grid-template-columns: 340px minmax(0, 1fr) 320px;
    gap: 0.85rem;
    flex: 1;
    min-height: 0;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 0.85rem;
    background: #252830;
    border: 1px solid #2d323b;
    border-radius: 8px;
    min-height: 0;
    overflow: auto;
  }

  .sidebar-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .sidebar-actions {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .ocr-settings {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    padding: 0.65rem;
    background: #1a1d23;
    border: 1px solid #2d323b;
    border-radius: 6px;
  }

  .settings-title {
    margin: 0;
    font-size: 0.82rem;
    font-weight: 600;
    color: #bdc1c6;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.82rem;
    color: #bdc1c6;
  }

  .field input[type="text"] {
    padding: 0.45rem 0.55rem;
    border: 1px solid #3c4043;
    border-radius: 4px;
    background: #1a1d23;
    color: #e8eaed;
  }

  .field input[type="text"]:focus {
    outline: none;
    border-color: #8ab4f8;
  }

  .range-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .range-row input[type="range"] {
    flex: 1;
    accent-color: #8ab4f8;
  }

  .range-value {
    min-width: 1.5rem;
    text-align: center;
    font-weight: 600;
    color: #e8eaed;
  }

  .field-hint {
    font-size: 0.75rem;
    color: #9aa0a6;
  }

  .gpu-hint {
    margin-top: -0.25rem;
  }

  .toggle-field {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.85rem;
    color: #e8eaed;
    cursor: pointer;
  }

  .toggle-field input {
    accent-color: #8ab4f8;
  }

  .folder-info {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
  }

  .folder-info .label {
    color: #9aa0a6;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .folder-info .path {
    word-break: break-all;
    color: #bdc1c6;
    line-height: 1.3;
  }

  .message {
    padding: 0.55rem 0.65rem;
    border-radius: 4px;
    font-size: 0.82rem;
    line-height: 1.4;
  }

  .message.info {
    background: #1e3a5f;
    color: #8ab4f8;
  }

  .message.error {
    background: #5c2b2b;
    color: #f28b82;
  }

  .ocr-progress-detail {
    margin-top: 0.35rem;
    font-size: 0.75rem;
    opacity: 0.85;
  }

  .panel {
    display: flex;
    flex-direction: column;
    background: #252830;
    border: 1px solid #2d323b;
    border-radius: 8px;
    min-height: 0;
    overflow: hidden;
  }

  .panel-subheader {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid #2d323b;
    flex-shrink: 0;
  }

  .panel-subheader h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .image-search {
    display: block;
    padding: 0.5rem 0.85rem;
    border-bottom: 1px solid #2d323b;
    flex-shrink: 0;
  }

  .image-search input {
    width: 100%;
    padding: 0.45rem 0.6rem;
    border: 1px solid #3c4043;
    border-radius: 6px;
    background: #1a1d23;
    color: #e8eaed;
    font-size: 0.85rem;
  }

  .image-search input::placeholder {
    color: #9aa0a6;
  }

  .image-search input:focus {
    outline: none;
    border-color: #8ab4f8;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .panel-badges {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .count-badge,
  .text-badge,
  .no-text-badge,
  .excluded-badge {
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .atlas-badge {
    padding: 0.15rem 0.45rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #1e3a5f;
    color: #8ab4f8;
  }

  .count-badge {
    background: #3c4043;
    color: #e8eaed;
  }

  .text-badge {
    background: #1e3a2f;
    color: #81c995;
  }

  .no-text-badge {
    background: #3c4043;
    color: #9aa0a6;
  }

  .excluded-badge {
    background: #4a3728;
    color: #fdd663;
  }

  .sidebar-right {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-right .sidebar-actions {
    flex-shrink: 0;
  }

  .panel-content {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: #9aa0a6;
  }

  .empty-state p {
    margin: 0;
  }

  .export-summary {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.55rem 0.65rem;
    background: #1a1d23;
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    color: #bdc1c6;
  }

  .summary-row strong {
    color: #e8eaed;
  }

  .btn {
    padding: 0.5rem 0.75rem;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
    font-size: 0.85rem;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-block {
    width: 100%;
  }

  .btn-primary {
    background: #8ab4f8;
    color: #1a1d23;
  }

  .btn-primary:hover:not(:disabled) {
    background: #aecbfa;
  }

  .btn-secondary {
    background: #3c4043;
    color: #e8eaed;
    border-color: #5f6368;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #5f6368;
  }

  .btn-export {
    background: #1e3a2f;
    color: #81c995;
    border-color: #3d5a45;
  }

  .btn-export:hover:not(:disabled) {
    background: #2d4a3a;
  }
</style>
