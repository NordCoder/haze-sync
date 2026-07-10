# W1-GDA-P8 — Delete candidate guardrails and mass-delete safety

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: implementation-worker

Work only through the GitHub connector. Do not merge the PR or change its lifecycle state.

## Context

GDA-P7 implementation, clean-code review, and artifact-based CI corrections are accepted. Final source CI is green.

- code_bearing_sha: f282fc6292826886520678e593b332bf3a46225b
- workflow: Component CI
- workflow_run_id: 29120606283
- run_number: 1539
- conclusion: success

The next implementation phase is GDA-P8 from crates/haze-gdrive-adapter/docs/implementation-plan.md.

## Read

Read the project process files, current GDrive control files, GDA-P8 plan section, existing scan/change-feed/export/mapping/echo abstractions, accepted Core/API delete-guard contracts, accepted Storage state boundaries, and PR diff. Do not read diagnostics artifacts unless a later fixer prompt explicitly requires them.

## Task

Implement conservative Drive delete-candidate handling and mass-delete protection.

- represent missing Drive files as candidates with safe timestamps/state rather than immediate deletes;
- require confirmation across accepted scans or change cycles before delete submission;
- distinguish disappearance from permission loss, folder movement, provider failure, and incomplete scans;
- integrate accepted Core/API delete-guard semantics and adapter-specific count/ratio thresholds without replacing Core policy;
- block and surface unsafe mass-delete conditions;
- support manual unlock only through an already accepted audited contract; otherwise keep it unavailable;
- add fake-provider tests for folder disappearance, auth scope loss, repeated absence, recovery, and mass-delete scenarios.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No immediate tombstone on first disappearance, Drive hard delete, adapter-local policy divergent from Core, unaudited manual override, live credentials/provider wiring, direct DB ownership, deployment files, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

Source/test/docs commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-gdrive-adapter/control/report.md using REPORT_TYPE IMPLEMENTATION and phase_id GDA-P8.
