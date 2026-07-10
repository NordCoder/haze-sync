# Cross-component contract evidence

component_owner: server
reported_by_component: storage
archived_at: 2026-07-11T00:30:00Z
storage_phase: STOR-P9C
storage_report_blob_sha: 40312e565b05832b7a450ba396b1c48f319b49e0
storage_code_bearing_sha: aa59064d641f4850f7c70fa615e638b52613dd95
storage_ci_run: 29124956486
storage_ci_conclusion: success
server_manifest_path: crates/haze-sync-server/Cargo.toml
server_manifest_blob_sha: bbc69c45584447d516a058db0cadee75ae6608ce

Observed violation:
`haze-sync-storage = { path = "../haze-sync-storage", features = ["test-support"] }` is declared under Server normal dependencies, which enables Storage test-only APIs in normal production compilation.

Required ownership correction:
keep the normal dependency feature-free and enable `test-support` only for Server dev/test targets or a dedicated test harness, preserving Server tests and full workspace CI.
