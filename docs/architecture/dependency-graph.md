# Component Dependency Graph

Initial high-level dependency graph:

~~~text
common
  -> core
  -> api
  -> storage
  -> server
  -> cli
  -> worktree
  -> gdrive-adapter

core + api + storage -> server
core -> cli
server/API contracts -> worktree
server/API contracts -> gdrive-adapter
server/API contracts -> obsidian-plugin
deployment/github-ci depend on all runtime crates for validation
~~~

## Boundary intent

- `common` owns shared value types.
- `core` owns pure behavior and no runtime side effects.
- `api` owns DTOs and passive route contracts.
- `storage` owns SQLx repository helpers and object store primitives.
- `server` owns runtime route wiring, auth execution, transactions, and object-store calls.
- Adapter components own provider/local integration loops only after contracts stabilize.
