# ACT-PR-01 — component-adaptive-card: Fix README + Add Executable README Tests

## Goal
Eliminate doc drift and ensure the README stays correct by making it executable in CI.

## Scope
### README fixes
- Correct commands:
  - `greentic-dev flow add-step` (not `greentic-dev add-step`)
- Ensure JSON examples are valid and consistent with current input schema.

### Add README gtests
Create `tests/gtests/README/` scenarios:
1) `01_quickstart_inline.gtest`
   - creates flow
   - adds step using README command
   - validates flow
2) `02_catalog_render_validate.gtest`
   - uses catalog card example with `asset_registry`
   - runs renderAndValidate
   - asserts `validation_issues` is empty
3) `03_interaction_submit.gtest`
   - sends `interaction` input
   - asserts `event.interaction_type == Submit`
   - asserts state updated (via state-store)

### CI
- Add a PR job that runs README gtests.

## Implementation details
- Use `greentic-integration-tester` to run these scenarios.
- Store generated flows and captured outputs under scenario artifacts.

## Acceptance criteria
- README examples pass in CI.
- If README commands drift, CI fails with clear output.

## Test plan
- Run locally via:
  - `greentic-integration-tester run --gtest tests/gtests/README/01_quickstart_inline.gtest`

## Notes
Treat README tests as non-negotiable PR gate.
