#!/usr/bin/env bash
# Development-seed tuning grid (seeds 1-3 only; results are excluded from evidence).
# Usage: scripts/dev_grid.sh OUT_DIR TICKS
set -u
OUT=${1:-results/dev_grid}
TICKS=${2:-200000}
BIN=./target/release
mkdir -p "$OUT"
for seed in 1 2 3; do
  for gin in 0.5 1.0 2.0; do
    wmax=$(python -c "print(6*$gin)")
    for eo in 0.02 0.05; do
      for arm in full readout frozen; do
        d="$OUT/s${seed}_gin${gin}_eo${eo}_${arm}"
        if [ ! -f "$d/summary.json" ]; then
          $BIN/life-run run --out "$d" --arm $arm --world-seed $seed --ticks $TICKS --ckpt-every 100000 --state-every 100 \
            --g-in $gin --w-max-hidden $wmax --eta-out $eo > /dev/null || echo "FAILED $d"
        fi
      done
    done
  done
done
echo "grid done"
