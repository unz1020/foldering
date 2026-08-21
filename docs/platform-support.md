# Platform support

5-Level V0.2.1 targets **Windows 10/11 x64** and **macOS 12+** on Apple Silicon and Intel Macs.

## Shared behavior

The application uses Tauri's OS path resolver for Downloads/Desktop and Rust `Path` / `PathBuf` for filesystem operations. It does not hard-code `/Users/...`, drive letters, or path separators.

Review Mode has the same mutation rules on both platforms:

- only top-level regular files in enabled watch locations are candidates;
- symlink sources/destinations are rejected;
- the destination must already exist inside the Workspace;
- destination depth must be Level 1-5;
- `99_ARCHIVE` is excluded from normal moves;
- same-name overwrite is blocked;
- move is copy -> byte-count verification -> flush -> source removal;
- transaction logging and Undo are local.

## Windows

- Supported baseline: Windows 10/11 x64.
- Desktop shell: Microsoft WebView2 (normally already present on current Windows).
- GitHub build output: `.msi` and NSIS `.exe`.
- Unsigned test builds may trigger Microsoft Defender SmartScreen.
- Public distribution should add Authenticode code signing later.

## macOS

- Supported baseline: macOS 12+.
- GitHub workflow builds a universal binary for Apple Silicon + Intel.
- GitHub build output: `.app` and `.dmg`.
- Unsigned/non-notarized downloaded builds can be blocked by Gatekeeper. For now, source builds are the recommended Mac testing path.
- Public distribution requires Apple Developer ID signing and notarization.

## Filesystem notes

- OneDrive/iCloud redirected Desktop folders are resolved through the OS/Tauri path APIs where available.
- Files still locked by another process are not force-moved; the operation fails and leaves the original in place.
- Case-insensitive destination collisions are blocked by the underlying filesystem plus exclusive destination creation.
