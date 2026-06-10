export interface AtlasRegion {
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  rotated: boolean;
}

export interface AtlasInfo {
  config_path: string;
  config_type: "atlas" | "plist";
  texture_file_name?: string | null;
  texture_width?: number | null;
  texture_height?: number | null;
  regions: AtlasRegion[];
}

export interface ScannedImage {
  path: string;
  relative_path: string;
  has_text?: boolean | null;
  detected_text?: string;
  atlas?: AtlasInfo | null;
}

export interface OcrImageResult {
  path: string;
  has_text: boolean;
  detected_text: string;
}

export interface OcrProgress {
  current: number;
  total: number;
  path: string;
}

export interface OcrSettings {
  cpu_cores: number;
  use_gpu: boolean;
}

export interface RegionOcrResult {
  name: string;
  has_text: boolean;
  detected_text: string;
}

export type OcrLogEntry =
  | {
      kind: "milestone";
      current: number;
      total: number;
      text_count: number;
    }
  | {
      kind: "text";
      index: number;
      total: number;
      path: string;
      detected_text: string;
    };
