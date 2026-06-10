import type { OcrImageResult, ScannedImage } from "../types";

export const DEFAULT_EXCLUDE_TERMS = ["library", "temp", "build"];

export function defaultExcludeInput(): string {
  return DEFAULT_EXCLUDE_TERMS.join(", ");
}

export function parseExcludeTerms(input: string): string[] {
  const fromInput = input
    .split(",")
    .map((term) => term.trim())
    .filter((term) => term.length > 0);

  if (fromInput.length > 0) {
    return fromInput;
  }

  return [...DEFAULT_EXCLUDE_TERMS];
}

export function shouldExcludePath(path: string, excludeTerms: string[]): boolean {
  if (excludeTerms.length === 0) return false;
  const normalized = path.toLowerCase();
  return excludeTerms.some((term) => normalized.includes(term.toLowerCase()));
}

export function filterImages(
  images: ScannedImage[],
  excludeTerms: string[],
): ScannedImage[] {
  return images.filter(
    (image) => !shouldExcludePath(image.relative_path, excludeTerms),
  );
}

/** Filter images by file name or relative path (case-insensitive). */
export function searchImages(
  images: ScannedImage[],
  query: string,
): ScannedImage[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return images;

  return images.filter((image) => {
    const relative = image.relative_path.toLowerCase();
    const fileName = image.relative_path.split(/[/\\]/).pop()?.toLowerCase() ?? "";
    return fileName.includes(needle) || relative.includes(needle);
  });
}

export type ExportMode = "images" | "with-atlas";

export function defaultExportName(projectName: string, mode: ExportMode): string {
  const trimmed = projectName.trim();
  const base = trimmed.length > 0 ? trimmed : "project";
  const suffix = mode === "with-atlas" ? "with-atlas" : "images";
  return `${base}-${suffix}.zip`;
}

export interface ExportZipEntry {
  path: string;
  relative_path: string;
}

function pathRelativeToRoot(absPath: string, rootDir: string): string {
  const normRoot = rootDir.replace(/\\/g, "/").replace(/\/$/, "");
  const normAbs = absPath.replace(/\\/g, "/");
  const prefix = `${normRoot}/`;
  if (normAbs.toLowerCase().startsWith(prefix.toLowerCase())) {
    return normAbs.slice(prefix.length);
  }
  return normAbs.split(/[/\\]/).pop() ?? normAbs;
}

/** Build zip entries for selected images, optionally including paired atlas/plist configs. */
export function buildExportEntries(
  images: ScannedImage[],
  rootDir: string,
  mode: ExportMode,
): ExportZipEntry[] {
  const entries: ExportZipEntry[] = [];
  const seen = new Set<string>();

  const add = (path: string, relativePath: string) => {
    const key = path.replace(/\\/g, "/").toLowerCase();
    if (seen.has(key)) return;
    seen.add(key);
    entries.push({
      path,
      relative_path: relativePath.replace(/\\/g, "/"),
    });
  };

  for (const image of images) {
    add(image.path, image.relative_path);

    if (mode === "with-atlas" && image.atlas?.config_path) {
      add(
        image.atlas.config_path,
        pathRelativeToRoot(image.atlas.config_path, rootDir),
      );
    }
  }

  return entries;
}

export function applyOcrResults(
  images: ScannedImage[],
  results: Array<Pick<OcrImageResult, "path" | "has_text" | "detected_text">>,
): ScannedImage[] {
  const resultByPath = new Map(results.map((result) => [result.path, result]));

  return images.map((image) => {
    const result = resultByPath.get(image.path);
    if (!result) return image;

    return {
      ...image,
      has_text: result.has_text,
      detected_text: result.detected_text,
    };
  });
}
