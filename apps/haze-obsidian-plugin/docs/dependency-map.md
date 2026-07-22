# Dependency Map: obsidian-plugin

## Component role in dependency graph

`apps/haze-obsidian-plugin` is the Obsidian client adapter for Haze Sync.

The plugin owns Obsidian-side UI/client behavior, vault-local event observation, safe local state, HTTP client calls to Haze Sync Server, and user-facing sync/status surfaces inside Obsidian.

The plugin does not own Core sync policy, API DTO definitions, Server route execution, Storage persistence, Google Drive provider behavior, Worktree runtime behavior, Deployment automation, or CI workflow policy.

## Independent development model

`apps/haze-obsidian-plugin` can be developed independently inside the `component/obsidian-plugin` branch.

The dependency map records client/adapter contracts and fan-in points. It does not impose a serial implementation order on Server, API, Core, Storage, GDrive adapter, Worktree, Deployment, or CI.

Allowed independent work includes:

- plugin settings UI and local state;
- Obsidian API integration boundaries;
- HTTP client abstraction against current API contracts;
- vault event normalization;
- status/error display;
- safe retry/offline UX foundations;
- TypeScript tests/build improvements inside the plugin component.

If the plugin needs new Server/API routes, Core semantics, Deployment URLs, or CI/plugin packaging behavior not currently contracted, it reports a contract-change request or fan-in need instead of implementing another component's responsibility.

## Upstream contracts consumed

The plugin may consume:

- Server HTTP endpoints;
- API request/response/error contracts;
- Obsidian plugin APIs as an external platform contract;
- Deployment-provided server URL/config guidance;
- GitHub CI Node/npm validation only as workflow-owned validation.

The plugin must not consume:

- Core internals;
- Storage internals or direct DB access;
- Google Drive provider APIs;
- Worktree local filesystem internals on the VPS;
- Server private handler state;
- deployment secrets.

## Downstream contracts exposed

Expected downstream users/consumers:

- users operating the Obsidian vault;
- Server/API, through client requests following public contracts;
- Deployment/docs, for plugin configuration guidance;
- GitHub CI, through plugin typecheck/build jobs;
- future QA/E2E scenarios.

Server/API must not depend on plugin internals for correctness.

## Forbidden dependency directions

The plugin must not:

- call Google Drive directly;
- decide final conflict/delete outcomes outside Core;
- write directly to the Haze Sync database;
- use Worktree files as hidden source-of-truth metadata;
- expose raw internal/provider/server errors to users;
- assume production server deployment details beyond configured URLs.

## Cross-component contracts

Important plugin contracts:

- the plugin is an external client/adapter, not sync authority;
- all sync decisions go through Server/API/Core;
- local vault events are normalized before being sent;
- UI output is safe and understandable;
- offline/retry behavior must not silently overwrite remote state;
- plugin packaging/build remains inside plugin/CI contracts.

## Integration/fan-in ownership

Fan-in is required when:

- API/Server routes used by the plugin change;
- Core conflict/tombstone semantics need client display;
- Deployment changes public endpoint/config guidance;
- CI changes plugin build/typecheck requirements;
- plugin UX needs new status/error fields.

These are integration gates. They do not block independent plugin work inside its component boundary.

## Dependency rules

- Plugin owns Obsidian-side behavior, not server policy.
- Plugin uses public HTTP/API contracts only.
- Plugin must never bypass Server/Core through provider or DB access.
- Plugin local state is client state, not source-of-truth metadata.
- Plugin tests should not require live server/provider deployment unless explicitly scoped as E2E.

## Contract-change notes

Current known contract questions:

1. Client route surface
   - Plugin may need additional API fields/routes for status, conflicts, and sync actions.
   - Missing fields/routes are API/Server/Core fan-in points.

2. Offline behavior
   - Plugin can implement UX/local queuing foundations.
   - Core/API owns final base-revision/conflict semantics.

3. Packaging and CI
   - Plugin owns source/build behavior.
   - github-ci owns workflow validation.

No serial implementation dependency is implied by this map.
