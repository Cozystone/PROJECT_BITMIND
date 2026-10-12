#!/usr/bin/env bash
# Second development grid: pulse-block input gain with held-block gain fixed at 0.5,
# readout rate 0.01/0.02. Seeds 1-3 only; excluded from evidence.
set -u
OUT=${1:-results/dev_grid2}
TICKS=${2:-200000}
BIN=./target/release
mkdir -p "$OUT"
for seed in 1 2 3; do
  for gin in 1.0 2.0 4.0; do
    wmax=$(python -c "print(max(3.0, 3*$gin))")
    for eo in 0.01 0.02; do
      for arm in full readout frozen; do
        d="$OUT/s${seed}_gp${gin}_eo${eo}_${arm}"
        if [ ! -f "$d/summary.json" ]; then
          $BIN/life-run run --out "$d" --arm $arm --world-seed $seed --ticks $TICKS --ckpt-every 100000 --state-every 100 \
            --g-in $gin --g-in-held 0.5 --w-max-hidden $wmax --eta-out $eo > /dev/null || echo "FAILED $d"
        fi
      done
    done
  done
done
echo "grid2 done"
