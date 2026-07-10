# W1-DEP-P6 — Reverse proxy, TLS, and public access boundary

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P5 implementation and clean-code review are accepted. Clean-code Component CI is green.

- code_bearing_sha: 0957601182378790ef72abea0e133572a6c044b1
- workflow: Component CI
- workflow_run_id: 29079825684
- run_number: 891
- conclusion: success

The next implementation phase is DEP-P6 from deploy/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, deployment docs/control files, accepted Server/API auth and upload-limit contracts, current deployment files, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement DEP-P6: Reverse proxy, TLS, and public access boundary.

Follow the plan:

- choose and document one secure-by-default reverse proxy example such as nginx or Caddy;
- document TLS termination and loopback/local upstream topology;
- document upload/body-size coordination without changing Server/API limits;
- document authenticated public-access expectations and which health surfaces may be exposed;
- document firewall and port boundaries;
- add placeholder-only proxy configuration under deploy/reverse-proxy/** only if it remains syntax-checkable and secret-free;
- document how operators validate configuration without committing certificate material.

## Allowed files

- deploy/docs/**
- deploy/reverse-proxy/**
- deploy/control/report.md

## Non-goals

No TLS private keys or certificates, public DNS automation, cloud-provider scripts, server auth-model changes, remote host automation, production credentials, workflow changes, or sibling component changes.

## CI trigger policy

Deployment/docs/config commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to DEP-P6.
