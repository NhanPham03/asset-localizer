import type { ScannedImage } from "../types";

const STORAGE_KEY = "asset-localizer-scanner-session";

export interface ScannerSession {
  rootDir: string | null;
  projectName: string;
  excludeInput: string;
  images: ScannedImage[];
  selectedPaths: string[];
}

export function loadSession(): ScannerSession | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as ScannerSession;
    if (!Array.isArray(parsed.images)) return null;
    return parsed;
  } catch {
    return null;
  }
}

export function saveSession(session: ScannerSession): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
  } catch {
    // Ignore quota / private mode errors
  }
}
