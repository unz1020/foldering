# ADR 0002: Schema before classification

Status: Accepted for V0.1

## Decision

Users define what each folder level means before any classifier is introduced.

## Consequences

- The core engine remains profession-agnostic.
- Presets are optional onboarding data, not hard-coded logic.
- Future AI output is constrained to a user-defined schema.
- Policy rules can reject a classification even when a model is confident.
