# 5-Level / Foldering

**Define how you work. 5-Level organizes files around it.**

`foldering` is the open-source repository for 5-Level, a local-first desktop file organizer for **Windows and macOS**. You define the meaning and order of your folder structure first, then Review Mode watches top-level files in Downloads/Desktop and recommends existing Workspace folders before any move happens.

> V0.2.1 is deliberately review-first: **no file moves because of a recommendation alone**. The user must approve each move. Approved moves are written to a local transaction log and can be undone.

## Platform support

- **Windows 10/11 x64** — source run + MSI/NSIS build workflow
- **macOS 12+** — Apple Silicon/Intel source run + universal app/DMG build workflow

See [`docs/platform-support.md`](docs/platform-support.md), [`docs/windows-quickstart-ko.md`](docs/windows-quickstart-ko.md), and [`docs/mac-quickstart-ko.md`](docs/mac-quickstart-ko.md).

## Core rules

1. Managed folders may be at most **5 levels deep** under a Workspace root.
2. Managed folders use `01_` through `98_`; `99` is reserved for `99_ARCHIVE`.
3. New folders are never created without user approval. V0.2.1 does not create folders at all.
4. Existing folder names, numbers, and locations are never changed automatically.
5. Uncertain classifications are never moved automatically.
6. Approved file moves never overwrite an existing same-name destination file.
7. Every approved move is recorded and supports Undo when the original path is available.
8. Work data is processed locally by default.

## Example

```text
Workspace: My Work          <- not counted as a level
└─ 01_Client                <- Level 1
   └─ 01_2026               <- Level 2
      └─ 01_Media           <- Level 3
         └─ 03_Channel      <- Level 4
            └─ 01_Assets    <- Level 5
               └─ file.jpg
```

## V0.2.1 scope

### Onboarding

- Workspace onboarding
- User-defined Folder Schema Builder (1-5 levels)
- Advertising-agency preset kept outside the core engine
- Native folder picker
- Existing folder health scan

### Review Mode

- Cross-platform Downloads/Desktop path resolution
- Non-recursive watcher: only top-level files are candidates
- Partial-download/temp-file filtering
- Filename + existing-folder-name matching
- Up to 3 existing destination recommendations
- Legacy/non-numbered existing paths can be shown but are flagged
- `99_ARCHIVE` is excluded from normal filing recommendations
- Explicit **Move approval** for every file move
- Manual existing-folder selection
- Destination must be inside the Workspace and within Level 1-5
- Same-name overwrite blocking
- No-overwrite move implemented as exclusive destination creation, full copy, byte verification, flush, then source removal
- Mutation rollback if transaction logging fails
- Local JSONL transaction log
- Undo with collision protection

## Not in V0.2.1 yet

- No automatic move mode
- No new-folder creation
- No automatic Archive movement
- No SHA-256 duplicate cleanup
- No PDF/PPTX/DOCX/XLSX content extraction yet
- No image understanding yet
- No external AI calls

These are intentionally separated into later milestones so the mutation boundary stays small and auditable.

## Tech stack

- **Desktop:** Tauri 2
- **UI:** React + TypeScript + Vite
- **Local filesystem engine:** Rust
- **File watcher:** `notify` 8.x
- **Package layout:** npm workspaces
- **License:** MIT

## Repository structure

```text
foldering/
├─ apps/
│  └─ desktop/
│     ├─ src/
│     └─ src-tauri/
│        └─ src/
│           ├─ config.rs
│           ├─ review.rs
│           ├─ scanner.rs
│           ├─ transactions.rs
│           └─ watcher.rs
├─ packages/
│  ├─ core/
│  └─ schema/
├─ presets/
├─ docs/
├─ scripts/
├─ LICENSE
├─ README.md
└─ CONTRIBUTING.md
```

The `core` package must never contain industry-specific concepts such as a brand, advertiser, ad platform, or campaign name. Those belong in presets or user data.

## Run locally

Prerequisites:

- Node.js 22+
- Rust stable toolchain
- Tauri platform prerequisites for Windows/macOS

Then:

```bash
npm install
npm run validate
npm run tauri:dev
```

## Build installers

Windows:

```powershell
npm run tauri --workspace @5level/desktop -- build --bundles msi,nsis
```

macOS:

```bash
npm run tauri --workspace @5level/desktop -- build --bundles app,dmg
```

GitHub Actions also contains **Build Installers**. Windows artifacts are MSI/NSIS; macOS builds are universal Apple Silicon + Intel bundles.

> Public binaries are not signed yet. Windows may show SmartScreen and macOS Gatekeeper may block downloaded builds. Public distribution should add Windows code signing and Apple Developer ID signing/notarization.

## Safety boundary

Review Mode accepts a source file only when it is a regular, non-symlink **top-level file** inside an enabled watch location. A destination must already exist inside the selected Workspace and be between Level 1 and Level 5. Existing destination files are never overwritten.

The transaction log and Workspace configuration are stored in Tauri's per-app local data directory.

## Roadmap

See [`docs/roadmap.md`](docs/roadmap.md).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). The project is designed so users from different professions can add presets without changing the core engine.
