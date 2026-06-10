<script lang="ts">
  import EditorPanel from "./components/EditorPanel.svelte";
  import ImageScanner from "./components/ImageScanner.svelte";
  import type { ScannedImage } from "./types";

  type ActiveTool = "scanner" | "editor";

  let activeTool = $state<ActiveTool>("scanner");
  let editorImage = $state<ScannedImage | null>(null);

  function openEditor(image: ScannedImage) {
    editorImage = image;
    activeTool = "editor";
  }

  function syncEditorImage(images: ScannedImage[]) {
    if (!editorImage) return;
    const updated = images.find((image) => image.path === editorImage!.path);
    if (updated) {
      editorImage = updated;
    }
  }
</script>

<main class="app">
  <header class="app-header">
    <div class="app-brand">
      <h1>Asset Localizer</h1>
      <p class="subtitle">
        Scan project folders for images, optionally detect text with OCR, and export assets
      </p>
    </div>

    <div class="tool-nav" role="tablist" aria-label="Tools">
      <button
        type="button"
        role="tab"
        class="tool-tab"
        class:active={activeTool === "scanner"}
        aria-selected={activeTool === "scanner"}
        onclick={() => (activeTool = "scanner")}
      >
        Scanner
      </button>
      <button
        type="button"
        role="tab"
        class="tool-tab"
        class:active={activeTool === "editor"}
        aria-selected={activeTool === "editor"}
        onclick={() => (activeTool = "editor")}
      >
        Editor
      </button>
    </div>
  </header>

  <div class="tool-content">
    <div class="tool-panel scanner-tool" class:tool-hidden={activeTool !== "scanner"}>
      <ImageScanner onOpenEditor={openEditor} onImagesChange={syncEditorImage} />
    </div>
    <div class="tool-panel editor-tool" class:tool-hidden={activeTool !== "editor"}>
      <EditorPanel image={editorImage} />
    </div>
  </div>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 1rem 1.25rem;
    gap: 0.85rem;
  }

  .app-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    flex-shrink: 0;
  }

  .app-brand h1 {
    margin: 0 0 0.25rem;
    font-size: 1.35rem;
    font-weight: 600;
  }

  .subtitle {
    margin: 0;
    color: #9aa0a6;
    font-size: 0.9rem;
  }

  .tool-nav {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .tool-tab {
    padding: 0.45rem 0.85rem;
    border: 1px solid #2d323b;
    border-radius: 6px;
    background: #252830;
    color: #9aa0a6;
    cursor: pointer;
    font-weight: 500;
    font-size: 0.9rem;
  }

  .tool-tab:hover {
    color: #e8eaed;
    background: #2d323b;
  }

  .tool-tab.active {
    color: #e8eaed;
    background: #3c4043;
    border-color: #5f6368;
  }

  .tool-content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .tool-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .tool-hidden {
    display: none;
  }

  .scanner-tool {
    min-height: 0;
  }

  .editor-tool {
    background: #252830;
    border: 1px solid #2d323b;
    border-radius: 8px;
    overflow: hidden;
  }
</style>
