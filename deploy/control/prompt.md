# W1-DEP-P2 — Deployment local compose hardening

Component: deployment
Component path: deploy
Branch: component/deployment
Base branch: main
Current main baseline SHA: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
Target branch: main

## Role

You are an Implementation Worker for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read before editing

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- docs-process/docs/development-model.md
- deploy/docs/component-contract.md
- deploy/docs/implementation-plan.md
- deploy/docs/implementation-log.md
- deploy/docs/dependency-map.md
- deploy/docs/decisions.md
- deploy/control/prompt.md
- current deploy/docker-compose.yml and .env.example if present

## Task

Implement phase DEP-P2 from the Deployment implementation plan: Local development compose hardening.

Goal: make local compose scaffolding clearer and safer for development-only dependency startup.

## Allowed component scope

```text
deploy/docker-compose.yml
deploy/docs/**
.env.example if local placeholders need documented alignment
deploy/control/report.md
```

## Expected work

- Verify PostgreSQL service healthcheck and local-only bind address.
- Align `.env.example` variables with compose and server config names only where safe and clear.
- Add optional local object-store/worktree bind directories only if Server/Worktree contracts are ready enough; otherwise document deferral.
- Add comments labeling local placeholders and non-production defaults.
- Add documented compose syntax validation and optional local startup smoke commands.

## Explicit non-goals

- No production credential material.
- No reverse proxy/TLS.
- No remote deploy.
- No real provider credential flow.
- No server/adapters services until startup contracts are stable.
- No product code changes.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires changing config variable names consumed by other components, adding non-local bind addresses, adding real credential material, or making local defaults production-like.

## Checks

Run applicable checks if available:

```text
docker compose -f deploy/docker-compose.yml config
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
deploy/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
