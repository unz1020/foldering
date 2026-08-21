# Folder Schema

A Folder Schema describes **what each level means**. It is not the same thing as the actual folder tree.

## Example schema

```text
Level 1 = Client
Level 2 = Year / Project
Level 3 = Work type
Level 4 = Channel
Level 5 = Detail
```

A real branch may stop early:

```text
01_Internal
└─ 01_Estimates
```

Another branch may use all five levels:

```text
01_Client
└─ 01_2026
   └─ 01_Media
      └─ 03_Channel
         └─ 01_Assets
```

## SchemaLevel fields

- `level`: integer 1-5
- `key`: stable machine key
- `name`: user-facing label
- `description`: what this level means
- `examples`: optional examples that help future classification

## Workspace

The Workspace root is outside the depth count.

```text
D:/Work/MyWorkspace      <- root, depth 0
└─ 01_Client             <- Level 1
```

## Presets

Presets are onboarding suggestions only. Selecting a preset does not create or rename any folder.
