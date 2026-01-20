# ACT-PR-02 — CI Workflows: Nightly Chaos + Failure Corpus

## Goal
Run the heavy suite nightly:
- full config matrix
- full negative suite
- fault injection matrix + concurrency
- save failure corpus for replay

## Scope
### Workflow: `adaptive-card-nightly-chaos.yml`
- schedule: nightly
- run with `--seed` and record it
- enable failure injection env vars
- store corpus:
  - `corpus/<date>/<seed>/<scenario>/trace.json`
  - inputs/outputs/logs

## Acceptance criteria
- Nightly produces a corpus artifact when failures occur.
- Replay instruction is printed and stored.
