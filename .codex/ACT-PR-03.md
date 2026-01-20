# ACT-PR-03 — component-adaptive-card: Config Matrix (Pairwise PR, Full Nightly)

## Goal
Systematically test the combinatorial configuration surface without hand-writing hundreds of tests.

## Scope
### Add matrix spec + generator
- `tests/fixtures/matrix_spec.yaml` defines axes:
  - card_source: inline/asset/catalog
  - mode: render/validate/renderAndValidate
  - bindings: handlebars/@{}/defaults/${expr}/mixed
  - assets: registry/env/missing
  - interaction: none/submit/showcard/toggle
  - node_id: none/present
- Add `tests/tools/gen_matrix` that emits:
  - input json cases into `tests/fixtures/generated/cases/*.json`
  - expected assertions into `tests/fixtures/generated/expect/*.yaml`
  - `.gtest` runners into `tests/gtests/matrix/*.gtest`

### Coverage strategy
- PR Gate: pairwise subset (fast)
- Nightly: full matrix + negative suite

## Implementation details
- Generator should support:
  - include/exclude constraints (e.g., asset_path only valid when card_source=asset)
  - expected outcomes (pass/fail + expected issue codes)

## Acceptance criteria
- At least 30 pairwise cases run in PR CI.
- Nightly can run 200+ cases without flakiness.

## Test plan
- Add a smoke run that generates then runs a small subset.

## Notes
Keep generator deterministic and checked-in outputs optional (prefer generate-at-test-time).
