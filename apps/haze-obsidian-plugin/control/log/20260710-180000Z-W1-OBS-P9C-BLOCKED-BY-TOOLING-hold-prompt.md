# W1-OBS-P9C-BLOCKED-BY-TOOLING — Node validation evidence gate

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: none

This is a hold notice, not an executable worker prompt.

## Accepted evidence

OBS-P9 implementation and OBS-P9C clean-code corrections are present. Final source Component CI is green.

- code_bearing_sha: d43ef5fe946de1f4467572ef0df5311e1b239717
- workflow: Component CI
- workflow_run_id: 29107572342
- run_number: 1374
- conclusion: success

## Tooling blocker

The current Component CI workflow does not execute or prove these plugin commands:

- npm test --workspace haze-obsidian-plugin
- npm run typecheck --workspace haze-obsidian-plugin
- npm run build --workspace haze-obsidian-plugin

GitHub-connector-only workers cannot execute these commands locally. Rust workspace CI must not be treated as Node validation evidence.

## Unblock condition

Provide independently observable successful execution evidence for all three Node commands in an accepted validation environment or add an explicitly authorized CI/fan-in validation path that runs them. If any command fails, route the failure through a scoped fixer prompt using the resulting evidence.

Do not launch a component worker from this hold notice.
