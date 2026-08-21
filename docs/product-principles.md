# Product Principles

## 1. Predictability over cleverness

The user should be able to predict where a file belongs before the AI can.

## 2. Five levels is a hard ceiling

The Workspace root is not counted. Managed folders below it may be at most five levels deep.

## 3. Existing structure first

The app searches and learns from existing folders before proposing new ones.

## 4. AI recommends; policy decides

Classification may be probabilistic. Safety rules are deterministic.

## 5. Approval before structure changes

The app may recommend a folder name, location, and number, but it may not create or reorganize folders without approval.

## 6. `99_ARCHIVE` has one meaning

`99` is reserved exclusively for archive folders.

## 7. Reversible by design

Every mutating action added after V0.1 must be represented as a transaction that can be inspected and, where possible, undone.

## 8. Local-first

Workspace data stays on the device unless the user explicitly enables an external service in a future version.
