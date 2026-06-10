<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { imageDisplayMetrics, pointInPolygon, regionTextureRect, regionToDisplay } from "../lib/atlasGeometry";
  import type { AtlasRegion, RegionOcrResult, ScannedImage } from "../types";

  interface Props {
    image: ScannedImage | null;
  }

  let { image }: Props = $props();

  let imgEl: HTMLImageElement | undefined = $state();
  let canvasEl: HTMLCanvasElement | undefined = $state();
  let hoveredRegion = $state<string | null>(null);
  let regionTexts = $state<Map<string, RegionOcrResult>>(new Map());
  let regionOcrLoading = $state(false);
  let regionOcrError = $state<string | null>(null);

  const regions = $derived(image?.atlas?.regions ?? []);
  const hasAtlas = $derived(regions.length > 0);

  const displayRegions = $derived.by(() => {
    if (!imgEl || !image || regions.length === 0) return [];

    const metrics = imageDisplayMetrics(
      imgEl.naturalWidth,
      imgEl.naturalHeight,
      imgEl.clientWidth,
      imgEl.clientHeight,
    );

    return regions.map((region) =>
      regionToDisplay(
        region,
        image!.atlas,
        imgEl!.naturalWidth,
        imgEl!.naturalHeight,
        metrics.scaleX,
        metrics.scaleY,
        metrics.offsetX,
        metrics.offsetY,
      ),
    );
  });

  function defaultOcrSettings() {
    return {
      cpu_cores: Math.max(1, navigator.hardwareConcurrency ?? 4),
      use_gpu: false,
    };
  }

  async function loadRegionOcr(current: ScannedImage) {
    if (!current.atlas || current.atlas.regions.length === 0) {
      regionTexts = new Map();
      regionOcrLoading = false;
      regionOcrError = null;
      return;
    }

    regionOcrLoading = true;
    regionOcrError = null;
    regionTexts = new Map();

    try {
      const results = await invoke<RegionOcrResult[]>("ocr_atlas_regions", {
        imagePath: current.path,
        regions: current.atlas.regions,
        configType: current.atlas.config_type,
        settings: defaultOcrSettings(),
      });

      const next = new Map<string, RegionOcrResult>();
      for (const result of results) {
        next.set(result.name, result);
      }
      regionTexts = next;
    } catch (err) {
      regionOcrError = err instanceof Error ? err.message : String(err);
    } finally {
      regionOcrLoading = false;
    }
  }

  function updateLayout() {
    drawRegions();
  }

  function drawRegions() {
    if (!canvasEl || !imgEl || !image) return;

    canvasEl.width = imgEl.clientWidth;
    canvasEl.height = imgEl.clientHeight;

    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, canvasEl.width, canvasEl.height);

    for (const display of displayRegions) {
      const isHovered = hoveredRegion === display.name;
      const [first, ...rest] = display.corners;
      if (!first) continue;

      ctx.beginPath();
      ctx.moveTo(first[0], first[1]);
      for (const corner of rest) {
        ctx.lineTo(corner[0], corner[1]);
      }
      ctx.closePath();

      ctx.fillStyle = isHovered ? "rgba(253, 214, 99, 0.18)" : "rgba(138, 180, 248, 0.12)";
      ctx.fill();

      ctx.strokeStyle = isHovered ? "#fdd663" : "#8ab4f8";
      ctx.lineWidth = isHovered ? 2 : 1;
      ctx.stroke();

      if (isHovered || displayRegions.length <= 16) {
        ctx.fillStyle = isHovered ? "#fdd663" : "#8ab4f8";
        ctx.font =
          '11px "Microsoft YaHei", "PingFang SC", "Noto Sans SC", "Segoe UI", system-ui, sans-serif';
        const label = display.rotated ? `${display.name} ↻` : display.name;
        ctx.fillText(label, first[0] + 2, first[1] + 12);
      }
    }
  }

  function handleCanvasMove(event: MouseEvent) {
    if (!canvasEl || displayRegions.length === 0) return;

    const rect = canvasEl.getBoundingClientRect();
    const mx = event.clientX - rect.left;
    const my = event.clientY - rect.top;

    let found: string | null = null;
    for (const display of displayRegions) {
      if (pointInPolygon(mx, my, display.corners)) {
        found = display.name;
        break;
      }
    }

    if (found !== hoveredRegion) {
      hoveredRegion = found;
      drawRegions();
    }
  }

  function setHovered(name: string | null) {
    if (hoveredRegion === name) return;
    hoveredRegion = name;
    drawRegions();
  }

  function regionTextFor(region: AtlasRegion): string | null {
    const result = regionTexts.get(region.name);
    if (result?.detected_text?.trim()) return result.detected_text.trim();
    return null;
  }

  $effect(() => {
    const current = image;
    if (!current) {
      regionTexts = new Map();
      regionOcrLoading = false;
      regionOcrError = null;
      return;
    }

    void loadRegionOcr(current);
  });

  $effect(() => {
    image;
    regions;
    displayRegions;
    hoveredRegion;
    requestAnimationFrame(updateLayout);
  });
</script>

<svelte:window onresize={updateLayout} />

<div class="editor">
  {#if !image}
    <div class="empty">
      <h2>Editor</h2>
      <p>Open an image from the scanner to inspect it here.</p>
    </div>
  {:else}
    <header class="editor-header">
      <div>
        <h2>{image.relative_path}</h2>
        {#if image.atlas}
          <span class="atlas-badge">
            {image.atlas.config_type.toUpperCase()} · {regions.length} region{regions.length === 1 ? "" : "s"}
          </span>
        {/if}
      </div>
    </header>

    <div class="editor-body">
      <section class="image-column">
        <div class="image-stage">
          <div class="image-wrap">
            <img
              bind:this={imgEl}
              src={convertFileSrc(image.path)}
              alt={image.relative_path}
              class="editor-image"
              onload={updateLayout}
            />
            {#if hasAtlas}
              <canvas
                bind:this={canvasEl}
                class="region-overlay"
                onmousemove={handleCanvasMove}
                onmouseleave={() => setHovered(null)}
              ></canvas>
            {/if}
          </div>
        </div>
      </section>

      <aside class="text-column">
        {#if hasAtlas}
          <div class="column-head">
            <span class="label">Regions</span>
            {#if regionOcrLoading}
              <span class="status">Running OCR on regions…</span>
            {:else if regionOcrError}
              <span class="status error">{regionOcrError}</span>
            {/if}
          </div>

          <ul class="region-text-list">
            {#each regions as region (region.name)}
              {@const text = regionTextFor(region)}
              {@const hasText = regionTexts.get(region.name)?.has_text}
              {@const tex = image ? regionTextureRect(region, image.atlas, imgEl?.naturalWidth ?? 0, imgEl?.naturalHeight ?? 0) : null}
              <li
                class:active={hoveredRegion === region.name}
                class:has-text={hasText}
                onmouseenter={() => setHovered(region.name)}
                onmouseleave={() => setHovered(null)}
              >
                <div class="region-head">
                  <span class="region-name">{region.name}</span>
                  {#if region.rotated}
                    <span class="rotated-tag">rotated</span>
                  {/if}
                </div>
                <div class="region-meta">
                  {#if tex}
                    {tex.x}, {tex.y} · {tex.width}×{tex.height}
                  {:else}
                    {region.x}, {region.y} · {region.width}×{region.height}
                  {/if}
                </div>
                {#if text}
                  <pre class="region-text">{text}</pre>
                {:else if regionOcrLoading}
                  <span class="region-placeholder">Scanning…</span>
                {:else}
                  <span class="region-placeholder muted">No text detected</span>
                {/if}
              </li>
            {/each}
          </ul>
        {:else if image.detected_text?.trim()}
          <div class="column-head">
            <span class="label">Detected text</span>
          </div>
          <pre class="full-text">{image.detected_text.trim()}</pre>
        {:else}
          <div class="column-head">
            <span class="label">Detected text</span>
          </div>
          <p class="no-text">No atlas regions and no OCR text for this image.</p>
        {/if}
      </aside>
    </div>
  {/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: 2rem;
    text-align: center;
    color: #9aa0a6;
  }

  .empty h2 {
    margin: 0 0 0.5rem;
    color: #e8eaed;
    font-size: 1.1rem;
  }

  .empty p {
    margin: 0;
  }

  .editor-header {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #2d323b;
    flex-shrink: 0;
  }

  .editor-header h2 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    word-break: break-all;
  }

  .atlas-badge {
    font-size: 0.75rem;
    color: #8ab4f8;
  }

  .editor-body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(220px, 0.8fr);
    gap: 0;
  }

  .image-column {
    min-height: 0;
    min-width: 0;
    display: flex;
    padding: 0.75rem;
    background: #1a1d23;
    border-right: 1px solid #2d323b;
  }

  .image-stage {
    position: relative;
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    max-height: calc(100vh - 9rem);
  }

  .image-wrap {
    position: relative;
    width: 100%;
    height: 100%;
    max-height: calc(100vh - 9rem);
    min-height: 0;
  }

  .editor-image {
    display: block;
    width: 100%;
    height: 100%;
    max-height: calc(100vh - 9rem);
    object-fit: contain;
  }

  .region-overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: auto;
  }

  .text-column {
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .column-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid #2d323b;
  }

  .label {
    font-size: 0.75rem;
    color: #9aa0a6;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .status {
    font-size: 0.72rem;
    color: #8ab4f8;
  }

  .status.error {
    color: #f28b82;
  }

  .region-text-list {
    margin: 0;
    padding: 0.5rem;
    list-style: none;
    overflow: auto;
    flex: 1;
    min-height: 0;
  }

  .region-text-list li {
    padding: 0.55rem 0.6rem;
    margin-bottom: 0.35rem;
    border: 1px solid #2d323b;
    border-radius: 6px;
    background: #252830;
    cursor: default;
  }

  .region-text-list li.active {
    border-color: #8ab4f8;
    background: #2d323b;
  }

  .region-text-list li.has-text {
    border-left: 3px solid #81c995;
  }

  .region-head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.2rem;
  }

  .region-name {
    color: #8ab4f8;
    font-size: 0.82rem;
    font-weight: 500;
    word-break: break-all;
  }

  .rotated-tag {
    font-size: 0.65rem;
    color: #fdd663;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .region-meta {
    font-size: 0.72rem;
    color: #9aa0a6;
    margin-bottom: 0.35rem;
  }

  .region-text,
  .full-text {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.82rem;
    color: #c8e6c9;
    line-height: 1.45;
  }

  .full-text {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0.75rem 0.85rem;
  }

  .region-placeholder {
    font-size: 0.78rem;
    color: #8ab4f8;
  }

  .region-placeholder.muted {
    color: #9aa0a6;
  }

  .no-text {
    margin: 0;
    padding: 0.75rem 0.85rem;
    color: #9aa0a6;
    font-size: 0.82rem;
  }
</style>
