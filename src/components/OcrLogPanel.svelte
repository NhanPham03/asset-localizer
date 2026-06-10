<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { OcrLogEntry } from "../types";

  interface Props {
    ocrRunning?: boolean;
  }

  let { ocrRunning = false }: Props = $props();

  let logs = $state<OcrLogEntry[]>([]);
  let logContainer: HTMLDivElement | undefined = $state();
  let autoScroll = $state(true);

  function fileName(path: string): string {
    return path.split(/[/\\]/).pop() ?? path;
  }

  function formatText(text: string): string {
    const trimmed = text.trim();
    return trimmed.length > 0 ? trimmed : "(empty)";
  }

  function logKey(entry: OcrLogEntry): string {
    if (entry.kind === "milestone") {
      return `milestone-${entry.current}`;
    }
    return `text-${entry.index}-${entry.path}`;
  }

  function scrollToBottom() {
    if (!autoScroll || !logContainer) return;
    requestAnimationFrame(() => {
      if (logContainer) {
        logContainer.scrollTop = logContainer.scrollHeight;
      }
    });
  }

  async function clearLogs() {
    await invoke("clear_ocr_logs_cmd");
    logs = [];
  }

  onMount(() => {
    invoke<OcrLogEntry[]>("get_ocr_logs").then((entries) => {
      logs = entries;
      scrollToBottom();
    });

    const cleanups: Array<() => void> = [];

    listen<OcrLogEntry>("ocr-log", (event) => {
      logs = [...logs, event.payload];
      scrollToBottom();
    }).then((cleanup) => cleanups.push(cleanup));

    listen("ocr-log-clear", () => {
      logs = [];
    }).then((cleanup) => cleanups.push(cleanup));

    return () => {
      for (const cleanup of cleanups) cleanup();
    };
  });
</script>

<section class="ocr-log-section">
  <div class="section-header">
    <h3 class="section-title">OCR Console</h3>
    <div class="header-actions">
      <label class="auto-scroll">
        <input type="checkbox" bind:checked={autoScroll} />
        Auto-scroll
      </label>
      <button type="button" class="btn-clear" onclick={clearLogs}>Clear</button>
    </div>
  </div>

  <div class="log-body" bind:this={logContainer}>
    {#if ocrRunning && logs.length === 0}
      <p class="empty">Scanning...</p>
    {:else if logs.length === 0}
      <p class="empty">Run OCR to see milestones and text detections here.</p>
    {:else}
      {#each logs as entry (logKey(entry))}
        {#if entry.kind === "milestone"}
          <article class="log-entry milestone">
            <span class="milestone-text">
              Scanned {entry.current}/{entry.total} images ({entry.text_count} with text)
            </span>
          </article>
        {:else}
          <article class="log-entry has-text">
            <div class="log-meta">
              <span class="log-index">[{entry.index}/{entry.total}]</span>
              <span class="log-file" title={entry.path}>{fileName(entry.path)}</span>
              <span class="log-status">TEXT</span>
            </div>
            <pre class="log-text">{formatText(entry.detected_text)}</pre>
          </article>
        {/if}
      {/each}
    {/if}
  </div>
</section>

<style>
  .ocr-log-section {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    min-height: 0;
    flex: 1;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .section-title {
    margin: 0;
    font-size: 0.82rem;
    font-weight: 600;
    color: #bdc1c6;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .auto-scroll {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.72rem;
    color: #9aa0a6;
    cursor: pointer;
  }

  .btn-clear {
    padding: 0.15rem 0.4rem;
    border: 1px solid #3c4043;
    border-radius: 4px;
    background: #1a1d23;
    color: #bdc1c6;
    font-size: 0.72rem;
    cursor: pointer;
  }

  .btn-clear:hover {
    background: #3c4043;
  }

  .log-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0.5rem;
    background: #1a1d23;
    border: 1px solid #2d323b;
    border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.72rem;
    line-height: 1.4;
  }

  .empty {
    margin: 0;
    color: #9aa0a6;
    font-family: Inter, system-ui, sans-serif;
    font-size: 0.78rem;
  }

  .log-entry {
    margin-bottom: 0.5rem;
    padding: 0.4rem 0.5rem;
    border-radius: 3px;
    border: 1px solid #2d323b;
    background: #252830;
  }

  .log-entry.milestone {
    border-color: #3c4043;
    background: #1e2430;
    text-align: center;
  }

  .milestone-text {
    color: #8ab4f8;
    font-weight: 600;
    font-family: Inter, system-ui, sans-serif;
    font-size: 0.78rem;
  }

  .log-entry.has-text {
    border-color: #3d5a45;
  }

  .log-meta {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin-bottom: 0.25rem;
    flex-wrap: wrap;
  }

  .log-index {
    color: #9aa0a6;
  }

  .log-file {
    color: #8ab4f8;
    word-break: break-all;
  }

  .log-status {
    margin-left: auto;
    font-size: 0.68rem;
    font-weight: 600;
    color: #81c995;
  }

  .log-text {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    color: #c8e6c9;
  }
</style>
