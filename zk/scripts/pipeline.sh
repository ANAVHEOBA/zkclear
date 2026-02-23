#!/usr/bin/env bash
set -euo pipefail

CIRCUIT=${1:-settlement_valid}
INPUT_FILE=${2:-"$(cd "$(dirname "$0")/.." && pwd)/fixtures/${CIRCUIT}.fixture.json"}

"$(dirname "$0")/compile.sh" "$CIRCUIT"
"$(dirname "$0")/setup.sh" "$CIRCUIT"
"$(dirname "$0")/prove.sh" "$CIRCUIT" "$INPUT_FILE"
"$(dirname "$0")/verify.sh" "$CIRCUIT"
"$(dirname "$0")/export_verifier.sh" "$CIRCUIT"

echo "full zk pipeline complete: $CIRCUIT"
