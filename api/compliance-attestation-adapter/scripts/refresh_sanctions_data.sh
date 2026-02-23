#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RAW_DIR="${ROOT_DIR}/data/raw"
OUT_JSON="${ROOT_DIR}/data/sanctions.json"
RAW_CSV="${RAW_DIR}/ofac_sdn.csv"

mkdir -p "${RAW_DIR}"

URL_PRIMARY="${OFAC_SDN_URL:-https://www.treasury.gov/ofac/downloads/sdn.csv}"
URL_FALLBACK="${OFAC_SDN_FALLBACK_URL:-https://sanctionssearch.ofac.treas.gov/SDN.csv}"

echo "downloading OFAC SDN CSV..."
if ! curl -fsSL "${URL_PRIMARY}" -o "${RAW_CSV}"; then
  echo "primary URL failed, trying fallback..."
  curl -fsSL "${URL_FALLBACK}" -o "${RAW_CSV}"
fi

echo "normalizing into ${OUT_JSON}..."
(
  cd "${ROOT_DIR}"
  OFAC_SDN_CSV_PATH="${RAW_CSV}" SANCTIONS_OUTPUT_PATH="${OUT_JSON}" cargo run --quiet --bin refresh_sanctions
)

echo "done: ${OUT_JSON}"
