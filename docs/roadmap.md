# Roadmap

## V0.1 — Schema + read-only health scan

- [x] Workspace onboarding
- [x] Folder Schema Builder
- [x] Preset architecture
- [x] Native folder selection
- [x] Read-only folder scanner
- [x] Core folder policy primitives
- [x] No mutation commands

## V0.2 / V0.2.1 — Review Mode + cross-platform baseline

- [x] Downloads/Desktop watcher
- [x] Top-level file candidate queue
- [x] filename/metadata candidate analysis
- [x] existing-folder matching
- [x] review-before-move UI
- [x] overwrite protection
- [x] transaction log
- [x] Undo
- [x] Workspace config stored locally
- [x] Windows 10/11 x64 baseline
- [x] macOS Apple Silicon/Intel baseline
- [x] Windows MSI/NSIS build workflow
- [x] macOS universal app/DMG build workflow
- [x] platform-specific test documentation

## V0.3 — Safe Auto Mode

- [ ] confidence threshold
- [ ] runner-up confidence gap
- [ ] existing-destination-only auto move
- [ ] managed-path-only requirement for Auto Mode
- [ ] user-confirmed switch from Review to Auto
- [ ] classification feedback learning

## V0.4 — Content understanding

- [ ] PDF/PPTX/DOCX/XLSX local metadata/text extraction
- [ ] image understanding option
- [ ] pluggable local model adapter
- [ ] optional external model adapter, disabled by default

## V0.5 — Archive + duplicate workflow

- [ ] SHA-256 duplicate detection
- [ ] safe source preference rules
- [ ] 90-day archive candidate discovery
- [ ] explicit project completion state
- [ ] archive approval workflow

## V0.6 — Folder migration assistant

- [ ] detailed health issue locations
- [ ] one-by-one numbering repair proposals
- [ ] 6+ Level flattening proposals
- [ ] approved folder rename/move transactions
- [ ] folder-structure Undo plan

## Distribution

- [x] CI repository validation
- [x] Rust safety tests on Linux/Windows/macOS
- [x] Windows/macOS installer artifact workflow
- [ ] signed Windows release
- [ ] signed/notarized macOS release
