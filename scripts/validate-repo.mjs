import { readFile, access } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const required = [
  "README.md",
  "LICENSE",
  "presets/advertising-agency.json",
  "packages/core/src/index.ts",
  "packages/schema/src/index.ts",
  "apps/desktop/src/App.tsx",
  "apps/desktop/src-tauri/src/scanner.rs",
  "apps/desktop/src-tauri/src/review.rs",
  "apps/desktop/src-tauri/src/transactions.rs",
  "apps/desktop/src-tauri/src/watcher.rs",
  "apps/desktop/src-tauri/src/config.rs",
  "docs/platform-support.md",
  "docs/windows-quickstart-ko.md",
  "docs/mac-quickstart-ko.md",
  ".github/workflows/build-installers.yml",
  ".github/workflows/ci.yml",
];

for (const item of required) {
  await access(path.join(root, item));
}

const preset = JSON.parse(
  await readFile(path.join(root, "presets/advertising-agency.json"), "utf8"),
);

const levels = preset?.schema?.levels;
if (!Array.isArray(levels) || levels.length < 1 || levels.length > 5) {
  throw new Error("Advertising preset must define 1-5 levels.");
}

for (let index = 0; index < levels.length; index += 1) {
  if (levels[index].level !== index + 1) {
    throw new Error(`Preset level sequence is invalid at index ${index}.`);
  }
}

const coreSource = await readFile(path.join(root, "packages/core/src/index.ts"), "utf8");
for (const forbidden of ["자코모", "교원웰스", "DV360", "Meta", "GFA"]) {
  if (coreSource.includes(forbidden)) {
    throw new Error(`Core package contains profession-specific value: ${forbidden}`);
  }
}

const transactionSource = await readFile(
  path.join(root, "apps/desktop/src-tauri/src/transactions.rs"),
  "utf8",
);
for (const safetyGuard of [
  "Destination must be an existing folder inside the Workspace.",
  "A file with the same name already exists in the destination.",
  "Source must be a top-level file in an enabled watch location.",
  "Undo was blocked because the original path is already occupied.",
  "Archive destinations are disabled in V0.2.1 Review Mode.",
  "Symlink destinations are not allowed in Review Mode.",
]) {
  if (!transactionSource.includes(safetyGuard)) {
    throw new Error(`V0.2.1 mutation guard is missing: ${safetyGuard}`);
  }
}

const reviewSource = await readFile(
  path.join(root, "apps/desktop/src-tauri/src/review.rs"),
  "utf8",
);
if (!reviewSource.includes("Archive is never a normal filing recommendation")) {
  throw new Error("Archive recommendation exclusion is missing.");
}

console.log("5-Level repository validation passed.");
console.log(`Preset levels: ${levels.length}`);
console.log("Core package: no advertising-specific examples detected.");
console.log("V0.2.1 mutation guard markers: present.");

const reviewPlatformSource = await readFile(
  path.join(root, "apps/desktop/src-tauri/src/review.rs"),
  "utf8",
);
for (const marker of ["download_dir()", "desktop_dir()", "PathBuf"]) {
  if (!reviewPlatformSource.includes(marker)) {
    throw new Error(`Cross-platform path marker is missing: ${marker}`);
  }
}
for (const forbiddenPath of ["/Users/", "C:\\\\Users\\"]) {
  if (reviewPlatformSource.includes(forbiddenPath)) {
    throw new Error(`Hard-coded OS user path found: ${forbiddenPath}`);
  }
}
console.log("Cross-platform path markers: present; no hard-coded user root detected.");
