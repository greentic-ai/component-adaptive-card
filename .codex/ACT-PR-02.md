# ACT-PR-02 — component-adaptive-card: Invocation Schema v1 + Warn-first Validation

## Goal
Make config errors discoverable early with a formal schema and stable validation issue codes.

## Scope
### Add schema
- `schemas/adaptive-card.invocation.v1.schema.json` covering:
  - `card_source` enum
  - `card_spec` shape (inline_json / asset_path / catalog_name + registry)
  - `mode` enum
  - `payload`, `session`, `state` (optional objects)
  - `interaction` shape
  - `envelope` (InvocationEnvelope v1 compatible metadata)

### Validation behavior
- On entry, validate input against schema.
- Add `validation_mode: off|warn|error` (default warn).
- Emit `validation_issues` with stable codes like:
  - `AC_INVOCATION_MISSING_FIELD`
  - `AC_INVOCATION_INVALID_TYPE`
  - `AC_INVOCATION_INVALID_ENUM`

## Implementation details
- Keep schema compatible with the validator you use elsewhere (Draft-07 recommended).
- Ensure errors include JSON pointer paths.

## Acceptance criteria
- Malformed inputs produce clear validation issues.
- In warn mode, component can still attempt render if safe.
- In error mode, component returns a stable error code.

## Test plan
- Add `tests/gtests/negative/schema/` scenarios:
  - missing `card_spec`
  - invalid `mode`
  - invalid `interaction` types

## Notes
Schema-first makes matrix testing maintainable.
