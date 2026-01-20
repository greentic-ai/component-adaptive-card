# ACT-PR-04 — component-adaptive-card: Negative Suite + Robust Error Codes

## Goal
Harden the component by treating “wrong answers” as first-class tests.

## Scope
### Add negative scenarios
- `tests/gtests/negative/`
  - `schema/` malformed invocation
  - `assets/` missing catalog, invalid registry, bad JSON
  - `bindings/` missing paths, invalid expressions, type mismatches
  - `validation/` invalid AdaptiveCard root/version/action shape
  - `interactions/` malformed interaction payloads

### Error code policy
Standardize stable error codes returned by component:
- `AC_SCHEMA_INVALID`
- `AC_ASSET_NOT_FOUND`
- `AC_ASSET_PARSE_ERROR`
- `AC_BINDING_EVAL_ERROR`
- `AC_CARD_VALIDATION_FAILED`
- `AC_INTERACTION_INVALID`

Ensure errors include:
- `code`
- `message`
- `details` (optional)

## Acceptance criteria
- Each negative test asserts error code + at least one validation issue.
- Failures produce trace.json (via runner or greentic-component test tool).

## Test plan
- Add at least 20 negative cases.

## Notes
This PR is where many real-world robustness improvements will land.
