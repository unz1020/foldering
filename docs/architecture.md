# Architecture

5-Level separates **user meaning**, **filesystem observation**, **recommendation**, and **mutation** so a classifier cannot directly change the user's folder structure.

```text
Folder Schema Builder
        ↓
Existing Folder Graph
        ↓
Candidate Analyzer / Matcher
        ↓
Review UI
        ↓ explicit user approval
Mutation Guard
        ↓
File Move
        ↓
Transaction Log / Undo
```

## Boundaries

### `packages/schema`
Defines a user's 1-5 level semantic schema. It contains no filesystem side effects.

### `packages/core`
Contains universal 5-Level constants and policy primitives. It must not contain profession-specific brands, clients, media platforms, or campaigns.

### `apps/desktop/src-tauri/src/scanner.rs`
Read-only Workspace health scan. Directory symlinks are never recursively followed.

### `apps/desktop/src-tauri/src/review.rs`
Builds the Review queue from regular top-level files in explicit watch locations. It matches filenames against existing folder labels locally. Archive paths are never normal filing recommendations.

### `apps/desktop/src-tauri/src/watcher.rs`
Uses the platform-recommended filesystem watcher and emits a lightweight refresh event to the UI. Watches are non-recursive.

### `apps/desktop/src-tauri/src/transactions.rs`
The only V0.2 file mutation boundary. Before moving it validates source, watch location, existing destination, Workspace containment, depth 1-5, and no overwrite. Moves are logged to local JSONL. It exclusively creates the destination, copies all bytes, verifies the byte count, flushes, then removes the source. If mutation logging fails, it attempts rollback.

### `apps/desktop/src-tauri/src/config.rs`
Stores the user Workspace configuration in the app-local data directory. The configuration is not sent to a remote service.

## V0.2 classification limitation
The current matcher uses filename-to-folder-label overlap, not document content understanding. Its percentage in the UI is a **path match score**, not an AI confidence score.
