#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ARTIFACTS_DIR="${1:-/tmp/matrix-smoke}"

"${ROOT}/tests/tools/gen_matrix" --mode pairwise

mkdir -p "${ARTIFACTS_DIR}"

greentic-integration-tester run \
  --gtest "${ROOT}/tests/gtests/matrix/pairwise/case_0001.gtest" \
  --artifacts-dir "${ARTIFACTS_DIR}"
