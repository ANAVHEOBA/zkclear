#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/common.sh"

CIRCUITS=("$@")
if [[ "${#CIRCUITS[@]}" -eq 0 ]]; then
  CIRCUITS=(settlement_valid compliance_valid rebate_valid)
fi

MAX_NONLINEAR_SETTLEMENT_VALID=${MAX_NONLINEAR_SETTLEMENT_VALID:-700}
MAX_LINEAR_SETTLEMENT_VALID=${MAX_LINEAR_SETTLEMENT_VALID:-80}
MAX_PROVE_SECONDS_SETTLEMENT_VALID=${MAX_PROVE_SECONDS_SETTLEMENT_VALID:-15}

MAX_NONLINEAR_COMPLIANCE_VALID=${MAX_NONLINEAR_COMPLIANCE_VALID:-200}
MAX_LINEAR_COMPLIANCE_VALID=${MAX_LINEAR_COMPLIANCE_VALID:-40}
MAX_PROVE_SECONDS_COMPLIANCE_VALID=${MAX_PROVE_SECONDS_COMPLIANCE_VALID:-10}

MAX_NONLINEAR_REBATE_VALID=${MAX_NONLINEAR_REBATE_VALID:-550}
MAX_LINEAR_REBATE_VALID=${MAX_LINEAR_REBATE_VALID:-40}
MAX_PROVE_SECONDS_REBATE_VALID=${MAX_PROVE_SECONDS_REBATE_VALID:-10}
FORCE_SETUP=${FORCE_SETUP:-0}

lower_to_upper() {
  echo "$1" | tr '[:lower:]' '[:upper:]'
}

get_budget() {
  local circuit="$1"
  local metric="$2"
  local key
  key="$(lower_to_upper "${metric}_${circuit}")"
  eval "echo \${${key}}"
}

parse_constraints() {
  local r1cs="$1"
  local info
  info="$(run_snarkjs r1cs info "$r1cs" 2>&1)"
  local nonlinear linear total
  nonlinear="$(echo "$info" | sed -n 's/.*non-linear constraints: *\([0-9]\+\).*/\1/p' | tail -n1)"
  linear="$(echo "$info" | sed -n 's/.*linear constraints: *\([0-9]\+\).*/\1/p' | tail -n1)"
  total="$(echo "$info" | sed -n 's/.*# of Constraints: *\([0-9]\+\).*/\1/p' | tail -n1)"

  if [[ -z "$nonlinear" || -z "$linear" ]]; then
    if [[ -n "$total" ]]; then
      nonlinear="$total"
      linear="0"
    else
      echo "error: failed to parse constraint counts for $r1cs" >&2
      echo "$info" >&2
      return 1
    fi
  fi
  echo "$nonlinear $linear"
}

echo "== ZK CI Check =="
echo "circuits: ${CIRCUITS[*]}"

for circuit in "${CIRCUITS[@]}"; do
  echo
  echo "-- circuit: $circuit --"

  "$(dirname "$0")/compile.sh" "$circuit"
  r1cs="$ARTIFACTS_DIR/$circuit/$circuit.r1cs"
  read -r nonlinear linear <<<"$(parse_constraints "$r1cs")"
  if [[ -z "$nonlinear" || -z "$linear" ]]; then
    echo "error: parsed empty constraint counts for $circuit" >&2
    exit 1
  fi
  max_nonlinear="$(get_budget "$circuit" "MAX_NONLINEAR")"
  max_linear="$(get_budget "$circuit" "MAX_LINEAR")"

  echo "constraints: non-linear=$nonlinear linear=$linear"
  echo "budgets: max_non-linear=$max_nonlinear max_linear=$max_linear"

  if (( nonlinear > max_nonlinear )); then
    echo "error: non-linear constraints exceeded for $circuit ($nonlinear > $max_nonlinear)" >&2
    exit 1
  fi
  if (( linear > max_linear )); then
    echo "error: linear constraints exceeded for $circuit ($linear > $max_linear)" >&2
    exit 1
  fi

  zkey="$ARTIFACTS_DIR/$circuit/$circuit.zkey"
  vkey="$ARTIFACTS_DIR/$circuit/$circuit.vkey.json"
  if [[ "$FORCE_SETUP" == "1" || ! -f "$zkey" || ! -f "$vkey" ]]; then
    "$(dirname "$0")/setup.sh" "$circuit"
  else
    echo "setup: reusing existing zkey/vkey for $circuit"
  fi

  start_ts=$(date +%s)
  "$(dirname "$0")/prove.sh" "$circuit" "$FIXTURES_DIR/$circuit.fixture.json"
  end_ts=$(date +%s)
  prove_seconds=$((end_ts - start_ts))

  max_prove="$(get_budget "$circuit" "MAX_PROVE_SECONDS")"
  echo "prove_time_seconds: $prove_seconds (budget: $max_prove)"
  if (( prove_seconds > max_prove )); then
    echo "error: proving time exceeded for $circuit ($prove_seconds > $max_prove)" >&2
    exit 1
  fi

  "$(dirname "$0")/verify.sh" "$circuit"
done

echo
echo "zk ci-check passed"
