# Contributing to 5-Level

Thanks for helping make 5-Level useful across different ways of working.

## Design rule

Keep **policy** separate from **profession-specific meaning**.

Core code may understand:

- Workspace
- Level 1-5
- `01`-`98`
- `99_ARCHIVE`
- approval requirements
- confidence thresholds
- reversible actions

Core code must not hard-code concepts such as:

- advertiser names
- brands
- media platforms
- designer workflows
- developer repository names

Those belong in presets or user configuration.

## Pull requests

- Keep changes small and reviewable.
- Add or update tests for policy behavior.
- Never weaken a safety rule silently.
- Any operation that mutates user files must include an Undo design.
- New presets should be data-only whenever possible.

## Presets

A preset describes a suggested Folder Schema. It does not create folders automatically.

Each preset should include:

- stable `id`
- display `name`
- short `description`
- 1-5 schema levels
- examples for each level

## Commit style

Simple conventional-style messages are encouraged:

```text
feat: add folder scanner summary
fix: reject non-archive 99 folders
docs: clarify workspace depth rules
```
