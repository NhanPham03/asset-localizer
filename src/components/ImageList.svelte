<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ScannedImage } from "../types";
  import ImagePreviewModal from "./ImagePreviewModal.svelte";

  interface Props {
    images: ScannedImage[];
    selectedPaths: Set<string>;
    ocrComplete: boolean;
    onToggle: (path: string) => void;
    onToggleAll: (checked: boolean) => void;
    onSelectWithText: () => void;
    onSelectWithoutText: () => void;
    onOpenEditor?: (image: ScannedImage) => void;
  }

  let {
    images,
    selectedPaths,
    ocrComplete,
    onToggle,
    onToggleAll,
    onSelectWithText,
    onSelectWithoutText,
    onOpenEditor,
  }: Props = $props();

  let selectAllInput: HTMLInputElement | undefined = $state();
  let previewImage = $state<ScannedImage | null>(null);

  const allSelected = $derived(
    images.length > 0 && images.every((image) => selectedPaths.has(image.path)),
  );
  const someSelected = $derived(
    images.some((image) => selectedPaths.has(image.path)),
  );
  const selectedCount = $derived(
    images.filter((image) => selectedPaths.has(image.path)).length,
  );
  const textCount = $derived(
    images.filter((image) => image.has_text === true).length,
  );
  const noTextCount = $derived(
    images.filter((image) => image.has_text === false).length,
  );

  $effect(() => {
    if (selectAllInput) {
      selectAllInput.indeterminate = someSelected && !allSelected;
    }
  });

  function textPreview(text: string | undefined): string {
    if (!text?.trim()) return "No text detected";
    const compact = text.replace(/\s+/g, " ").trim();
    return compact.length > 80 ? `${compact.slice(0, 80)}...` : compact;
  }

  function openPreview(_event: MouseEvent, image: ScannedImage) {
    previewImage = image;
  }
</script>

<ImagePreviewModal
  image={previewImage}
  onClose={() => (previewImage = null)}
  onOpenEditor={onOpenEditor
    ? (image) => {
        previewImage = null;
        onOpenEditor(image);
      }
    : undefined}
/>

{#if images.length === 0}
  <div class="empty-state">
    <p>No images found</p>
    <p class="hint">Pick a project folder to scan for images</p>
    <small>Supported formats: PNG, JPG, JPEG, GIF, BMP, WebP</small>
  </div>
{:else}
  <div class="list-toolbar">
    <div class="toolbar-left">
      <label class="select-all">
        <input
          bind:this={selectAllInput}
          type="checkbox"
          checked={allSelected}
          onchange={(event) => onToggleAll(event.currentTarget.checked)}
        />
        <span>Select all ({images.length})</span>
      </label>
      {#if ocrComplete}
        <div class="select-filters">
          <button
            type="button"
            class="filter-btn"
            onclick={onSelectWithText}
            disabled={textCount === 0}
          >
            With text ({textCount})
          </button>
          <button
            type="button"
            class="filter-btn"
            onclick={onSelectWithoutText}
            disabled={noTextCount === 0}
          >
            Without text ({noTextCount})
          </button>
        </div>
      {/if}
    </div>
    <span class="selected-count">{selectedCount} selected</span>
  </div>

  <div class="file-list">
    {#each images as image, index (image.path)}
      <div
        class="file-item"
        class:selected={selectedPaths.has(image.path)}
        class:no-text={image.has_text === false}
        class:has-text={image.has_text === true}
      >
        <span class="item-number" aria-hidden="true">{index + 1}</span>
        <button
          type="button"
          class="file-thumb"
          onclick={(event) => openPreview(event, image)}
          aria-label="View full image for {image.relative_path}"
          title="View full image"
        >
          <img src={convertFileSrc(image.path)} alt="" loading="lazy" />
        </button>
        <button
          type="button"
          class="file-main"
          onclick={() => onToggle(image.path)}
          aria-pressed={selectedPaths.has(image.path)}
        >
          <div class="file-meta">
            <span class="file-path" title={image.path}>{image.relative_path}</span>
            <div class="file-tags">
              {#if image.atlas && image.atlas.regions.length > 0}
                <span class="atlas-tag" title={image.atlas.config_path}>
                  {image.atlas.config_type} · {image.atlas.regions.length}
                </span>
              {/if}
              {#if image.has_text === true}
                <span class="text-label" title={image.detected_text}>
                  {textPreview(image.detected_text)}
                </span>
              {:else if image.has_text === false}
                <span class="no-text-label">No text detected</span>
              {/if}
            </div>
          </div>
          <span class="file-check" aria-hidden="true">
            {#if selectedPaths.has(image.path)}✓{/if}
          </span>
        </button>
        {#if onOpenEditor}
          <button
            type="button"
            class="editor-btn"
            onclick={() => onOpenEditor(image)}
            title="Open in Editor"
            aria-label="Open in Editor"
          >
            Edit
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1.5rem;
    text-align: center;
    color: #9aa0a6;
  }

  .empty-state p {
    margin: 0 0 0.35rem;
  }

  .hint {
    font-size: 0.9rem;
  }

  .empty-state small {
    margin-top: 0.5rem;
    font-size: 0.8rem;
    opacity: 0.8;
  }

  .list-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #2d323b;
    flex-shrink: 0;
  }

  .toolbar-left {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }

  .select-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .filter-btn {
    padding: 0.2rem 0.5rem;
    border: 1px solid #3c4043;
    border-radius: 4px;
    background: #252830;
    color: #bdc1c6;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .filter-btn:hover:not(:disabled) {
    background: #3c4043;
    color: #e8eaed;
  }

  .filter-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .select-all {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    cursor: pointer;
    font-size: 0.85rem;
    color: #bdc1c6;
  }

  .selected-count {
    font-size: 0.8rem;
    color: #9aa0a6;
  }

  .file-list {
    display: flex;
    flex-direction: column;
  }

  .file-item {
    display: flex;
    align-items: stretch;
    gap: 0.5rem;
    width: 100%;
    border-bottom: 1px solid #2d323b;
    background: transparent;
  }

  .item-number {
    flex-shrink: 0;
    align-self: center;
    width: 2.25rem;
    margin-left: 0.5rem;
    text-align: right;
    font-size: 0.78rem;
    font-weight: 600;
    color: #9aa0a6;
    font-variant-numeric: tabular-nums;
  }

  .file-item.has-text .item-number {
    color: #69f0ae;
  }

  .file-item:hover {
    background: #2d323b;
  }

  .file-item.selected {
    background: #32363f;
  }

  .file-main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 0;
    padding: 0.5rem 0.75rem 0.5rem 0;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .file-item.has-text {
    background: rgba(30, 58, 47, 0.55);
    border-left: 3px solid #4caf50;
  }

  .file-item.has-text:hover {
    background: rgba(30, 58, 47, 0.75);
  }

  .file-item.has-text.selected {
    background: rgba(46, 90, 72, 0.85);
  }

  .file-item.has-text .file-path {
    color: #69f0ae;
    font-weight: 600;
  }

  .file-item.no-text .file-path {
    color: #9aa0a6;
  }

  .file-thumb {
    flex-shrink: 0;
    width: 96px;
    height: 96px;
    margin: 0.5rem 0 0.5rem 0.75rem;
    padding: 0;
    border-radius: 4px;
    overflow: hidden;
    background: #1a1d23;
    border: 1px solid #3c4043;
    cursor: zoom-in;
  }

  .file-thumb:hover {
    border-color: #8ab4f8;
  }

  .file-thumb img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }

  .file-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .file-path {
    font-size: 0.85rem;
    word-break: break-all;
    line-height: 1.3;
  }

  .text-label {
    font-size: 0.78rem;
    color: #69f0ae;
    font-weight: 500;
    opacity: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .no-text-label {
    font-size: 0.78rem;
    color: #9aa0a6;
    font-style: italic;
  }

  .file-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: center;
  }

  .atlas-tag {
    font-size: 0.72rem;
    color: #8ab4f8;
    background: #1e3a5f;
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
  }

  .editor-btn {
    align-self: center;
    margin-right: 0.75rem;
    padding: 0.2rem 0.45rem;
    border: 1px solid #3c4043;
    border-radius: 4px;
    background: #252830;
    color: #8ab4f8;
    font-size: 0.72rem;
    cursor: pointer;
    flex-shrink: 0;
  }

  .editor-btn:hover {
    background: #3c4043;
  }
</style>
