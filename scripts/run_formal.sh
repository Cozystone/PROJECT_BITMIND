#!/usr/bin/env bash
# Formal seeds 101-110 of the first experiment. Refuses to run unless the freeze guard passes.
# Usage: scripts/run_formal.sh [OUT_ROOT] [PARALLEL]
set -u
ROOT=${1:-results/formal}
PAR=${2:-2}
powershell -ExecutionPolicy Bypass -File scripts/check_freeze_life.ps1 || { echo "freeze guard failed; formal run refused"; exit 1; }
mkdir -p "$ROOT"
git rev-parse HEAD > "$ROOT/COMMIT"
seeds=(101 102 103 104 105 106 107 108 109 110)
i=0
while [ $i -lt ${#seeds[@]} ]; do
  for ((j=0; j<PAR && i<${#seeds[@]}; j++)); do
    bash scripts/run_seed.sh ${seeds[$i]} "$ROOT" experiments/formal-config.json 1000000 > "$ROOT/seed${seeds[$i]}.log" 2>&1 &
    i=$((i+1))
  done
  wait
done
./target/release/life-lab gate --root "$ROOT" --seeds 101-110 | tee "$ROOT/gate.txt"
echo "formal run done"
