# ADR 0001: Tauri + React + TypeScript

Status: Accepted for V0.1

## Decision
Use Tauri 2 for the desktop shell and privileged local filesystem layer, with React + TypeScript + Vite for the UI.

## Why
- Windows and macOS are first-class targets.
- Local filesystem behavior can live in Rust instead of a browser sandbox.
- The UI remains approachable for open-source contributors familiar with web tooling.
- Tauri keeps the desktop shell smaller than an Electron-style bundled Chromium architecture.
- Future file mutations can be exposed as a deliberately small command surface behind the Policy Engine.

## V0.1 boundary
Only read-only scan commands are exposed. Mutation commands are intentionally absent.
