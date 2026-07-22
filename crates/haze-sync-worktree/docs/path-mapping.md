# Worktree Path Mapping

## Scope

WT-P2 introduces the safe mapping boundary between shared `VaultPath` values and local filesystem paths under one configured worktree root.

## Root containment

`WorktreeConfig` accepts one configured absolute root. Path mapping is lexical and conservative in this phase: it does not follow symlinks and does not inspect filesystem metadata. Later scanner/writer phases must keep rejecting symlinks and special files before importing or materializing content.

A `VaultPath` maps to a local path by appending its normalized forward-slash segments below the configured root. A local path maps back to a `VaultPath` only when it is component-contained under that same root.

Rejected cases include:

- empty or relative roots;
- root-only local paths;
- local paths outside the configured root;
- sibling-prefix escapes such as a path under `worktree-evil`;
- traversal components after the root prefix;
- local path segments containing backslashes;
- non-UTF-8 local path segments.

Errors expose safe categories only and do not include configured absolute roots.

## Reserved runtime paths

Worktree-owned runtime state is reserved under the top-level `_haze_runtime` directory inside the configured root.

Initial reserved subdirectories are:

```text
_haze_runtime/tmp
_haze_runtime/trash
_haze_runtime/metadata
_haze_runtime/echo
```

For compatibility with the shared `VaultPath` runtime-path policy, these local paths are also treated as reserved and are not mapped back to syncable vault paths:

```text
_haze_tmp/**
state/**
logs/**
trash/**
*.tmp
*.part
*.swp
```

`_haze_conflicts/**` is intentionally not reserved by Worktree path mapping, so Core conflict materialization remains possible.
