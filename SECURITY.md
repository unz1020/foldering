# Security

5-Level is a filesystem utility, so mutation safety is treated as a primary security boundary.

## V0.2 mutation rules

- Recommendations never cause a move by themselves.
- Only regular, non-symlink top-level files in enabled watch locations can be moved.
- The destination must already exist inside the selected Workspace.
- Destination depth must be Level 1 through Level 5.
- Existing destination files are never overwritten.
- `99_ARCHIVE` is not used as an ordinary recommendation target and V0.2 blocks moves into Archive paths.
- Approved moves are logged locally and can be undone when the original path is free.
- Directory symlinks are not followed by Workspace scans.

## Data handling

V0.2 uses local filenames, file metadata, folder names, and local transaction/config files. It does not make external AI calls.

## Reporting

If you find a path-escape, overwrite, symlink, data-loss, or Undo issue, please avoid publishing destructive reproduction details until a fix is available. Open a security report through the repository's private security reporting mechanism when available.
