# Archived active prompt

component: gdrive-adapter
wave: W1
phase: GDA-P5C
agent_role: clean-code-reviewer
source_path: crates/haze-gdrive-adapter/control/prompt.md
source_sha: f64cbb3e090694878ab83e504555275aab67a74c
archive_reason: GDA-P5C report closed the clean-code slot and CI requires fixer triage.

---

# W1-GDA-P5C — GDrive full-scan/import clean-code review

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P5 implementation and CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: 3641883fcf20425692516e6b93d6d5c858cdee8e
- workflow: Component CI
- workflow_run_id: 29084342761
- run_number: 1038
- conclusion: success

## Read

Read process sources, component docs/control files, current GDA-P5 source/tests, relevant accepted Common/API/Core boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review GDA-P5 full Drive subtree scan and Core import planning plus the formatter correction.

Focus on recursive traversal, path normalization, classification, content verification, base/null-base request planning, conservative delete candidates, adapter mode behavior, injected mapping boundaries, tests, and non-goal preservation.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No Drive export, live provider wiring beyond accepted abstractions, provider mutation, Core/API execution, direct DB ownership, immediate delete, workflow/dependency changes, or sibling-component changes.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id GDA-P5C.
