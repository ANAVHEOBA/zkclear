#!/usr/bin/env bash
set -euo pipefail

MODE=${1:-fast}

echo "== ZK Regression =="
echo "mode: $MODE"

cd "$(dirname "$0")/.."

echo
echo "-- rust constraint regression tests --"
cargo test

if [[ "$MODE" == "full" ]]; then
  echo
  echo "-- full circuit pipeline regression --"
  ./scripts/pipeline.sh settlement_valid fixtures/settlement_valid.fixture.json
  ./scripts/pipeline.sh compliance_valid fixtures/compliance_valid.fixture.json
  ./scripts/pipeline.sh rebate_valid fixtures/rebate_valid.fixture.json
fi

echo
echo "zk regression passed"
