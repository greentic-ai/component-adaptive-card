# ACT-PR-05 — component-adaptive-card: Trace Enrichment (Bindings, Assets, State, Interaction)

## Goal
Make adaptive-card failures easy to debug by adding component-specific trace fields.

## Scope
When tracing is enabled (runner or component test tool), record:
- `card_source`
- `asset_resolution`:
  - mode: inline/host/wasm
  - resolved path/logical name
  - asset hash
- `bindings_summary`:
  - number of handlebars expansions
  - number of @{path} replacements
  - number of ${expr} evaluations
  - missing-path warnings count
- `interaction_summary`:
  - type
  - action_id
  - card_instance_id
  - route metadata (if any)
- `state_summary`:
  - node_id scope key
  - state read hash
  - state write hash

## Implementation details
- Ensure no sensitive data is written into trace by default.
  - Store hashes and counts rather than full payload.
  - Provide `--trace-capture-inputs` only for test/nightly.

## Acceptance criteria
- A failing binding/asset/interaction test produces a trace that points to the failure class.

## Test plan
- Add one test per category that asserts trace contains key fields.

## Notes
This PR amplifies the value of Phase 0 replay.
