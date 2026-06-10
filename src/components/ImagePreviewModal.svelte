<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { ScannedImage } from "../types";

  interface Props {
    image: ScannedImage | null;
    onClose: () => void;
    onOpenEditor?: (image: ScannedImage) => void;
  }

  let { image, onClose, onOpenEditor }: Props = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if image}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={onClose} role="presentation">
    <div
      class="dialog"
      onclick={(event) => event.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <header class="dialog-header">
        <div class="dialog-title">
          <h2>{image.relative_path}</h2>
          {#if image.has_text === true}
            <span class="badge text">Text detected</span>
          {:else if image.has_text === false}
            <span class="badge none">No text</span>
          {/if}
        </div>
        <div class="dialog-actions">
          {#if onOpenEditor}
            <button
              type="button"
              class="editor-btn"
              onclick={() => onOpenEditor(image)}
            >
              Open in Editor
            </button>
          {/if}
          <button type="button" class="close-btn" onclick={onClose} aria-label="Close preview">
            ×
          </button>
        </div>
      </header>

      <div class="preview-frame">
        <img
          src={convertFileSrc(image.path)}
          alt={image.relative_path}
          class="preview-image"
        />
      </div>

      {#if image.detected_text?.trim()}
        <div class="detected-text">
          <span class="label">Detected text</span>
          <pre>{image.detected_text.trim()}</pre>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.72);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    width: min(960px, 100%);
    max-height: min(90vh, 900px);
    background: #252830;
    border: 1px solid #3c4043;
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.45);
  }

  .dialog-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #2d323b;
    flex-shrink: 0;
  }

  .dialog-title h2 {
    margin: 0 0 0.35rem;
    font-size: 0.9rem;
    font-weight: 600;
    word-break: break-all;
  }

  .badge {
    display: inline-block;
    padding: 0.1rem 0.45rem;
    border-radius: 999px;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .badge.text {
    background: #1e3a2f;
    color: #81c995;
  }

  .badge.none {
    background: #3c4043;
    color: #9aa0a6;
  }

  .dialog-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .editor-btn {
    padding: 0.35rem 0.65rem;
    border: 1px solid #3c4043;
    border-radius: 4px;
    background: #252830;
    color: #8ab4f8;
    font-size: 0.82rem;
    cursor: pointer;
  }

  .editor-btn:hover {
    background: #3c4043;
  }

  .close-btn {
    flex-shrink: 0;
    width: 2rem;
    height: 2rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #bdc1c6;
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
  }

  .close-btn:hover {
    background: #3c4043;
    color: #e8eaed;
  }

  .preview-frame {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: #1a1d23;
    overflow: auto;
  }

  .preview-image {
    max-width: 100%;
    max-height: min(65vh, 640px);
    width: auto;
    height: auto;
    object-fit: contain;
    display: block;
  }

  .detected-text {
    flex-shrink: 0;
    padding: 0.75rem 1rem;
    border-top: 1px solid #2d323b;
    max-height: 160px;
    overflow: auto;
  }

  .detected-text .label {
    display: block;
    margin-bottom: 0.35rem;
    font-size: 0.75rem;
    color: #9aa0a6;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .detected-text pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.82rem;
    color: #c8e6c9;
  }
</style>
