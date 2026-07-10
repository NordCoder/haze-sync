# API compatibility fixture provenance

`api-contract-v1.json` is a component-local snapshot of the accepted language-neutral
API fixture owned by `crates/haze-sync-api`.

- API acceptance commit: `3109c0fd9b456ca5fd8db099cd83843dae44cef9`
- Canonical fixture path: `crates/haze-sync-api/fixtures/api-contract-v1.json`
- Canonical fixture blob: `ac69d26d4de689595dd21381a6aec2ed03369fa0`
- Schema version: `1`

The plugin does not own or redefine this contract. The snapshot exists so the component
branch can run TypeScript compatibility tests before API branch fan-in. When the canonical
workspace fixture is present, `api-compatibility.test.ts` requires the parsed JSON values to
be identical.

Update this snapshot only after an accepted API contract/fixture change. Never add tokens,
real server URLs, production identifiers, provider payloads, real vault content, or other
runtime data.
