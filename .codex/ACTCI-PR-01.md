# ACT-PR-01 — CI Workflows: PR Gate for Adaptive Card Testing

## Goal
Add a fast PR workflow that:
- runs README tests
- runs pairwise matrix subset
- runs negative smoke subset
- uploads artifacts (trace/logs) on failure

## Scope
### Workflow: `adaptive-card-pr-gate.yml`
Steps:
- checkout
- toolchain install
- build
- run:
  - `greentic-integration-tester run` on `tests/gtests/README/*.gtest`
  - `.../matrix/pairwise/*.gtest`
  - `.../negative/smoke/*.gtest`
- on failure:
  - upload artifacts directory
  - (optional) comment summary on PR

## Acceptance criteria
- PRs touching adaptive-card or runner/tester trigger this workflow.
- Failures upload trace.json and logs.
