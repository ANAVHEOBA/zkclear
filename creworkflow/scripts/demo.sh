#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/3] Checking workspace"
cargo check -p orchestrator

echo "[2/3] Running CRE simulation"
./scripts/cre-cli-simulate.sh

echo "[3/3] Generating E2E artifact"
./scripts/generate-e2e-artifact.sh

echo "Demo complete."
echo "Artifact: $ROOT_DIR/artifacts/e2e/full-run/manifest.json"
