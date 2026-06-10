import { spawnSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dirname, "..");

/** Paths safe to delete; all are recreated by install/build. */
const paths = [
  "dist",
  "dist-ssr",
  "node_modules",
  "src-tauri/target",
  "src-tauri/gen",
  "src-tauri/ocr-models",
];

function removePath(absPath, rel) {
  if (!existsSync(absPath)) return false;

  try {
    rmSync(absPath, {
      recursive: true,
      force: true,
      maxRetries: 10,
      retryDelay: 200,
    });
    console.log(`removed ${rel}`);
    return true;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.warn(`failed to remove ${rel}: ${message}`);
    return false;
  }
}

function cargoClean() {
  const manifest = join(root, "src-tauri", "Cargo.toml");
  if (!existsSync(manifest)) return false;

  console.log("running cargo clean for src-tauri/target");
  const result = spawnSync("cargo", ["clean", "--manifest-path", manifest], {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32",
  });

  return result.status === 0;
}

let removed = 0;

for (const rel of paths) {
  const absPath = join(root, rel);
  if (rel === "src-tauri/target") {
    if (removePath(absPath, rel)) {
      removed += 1;
      continue;
    }
    if (cargoClean()) {
      removed += 1;
    }
    continue;
  }

  if (removePath(absPath, rel)) {
    removed += 1;
  }
}

if (removed === 0) {
  console.log("nothing to clean");
} else {
  console.log(`cleaned ${removed} path(s)`);
}
