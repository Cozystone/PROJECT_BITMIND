#!/usr/bin/env bash
# One seed of the first experiment: all arms, re-adaptation branches at every structural change
# of the full life, per-seed evaluation. Deterministic given the seed and the config file.
# Usage: scripts/run_seed.sh SEED OUT_ROOT CONFIG_JSON [TICKS]
set -u
SEED=$1
ROOT=$2
CFG=$3
TICKS=${4:-1000000}
BIN=./target/release
OUT="$ROOT/seed$SEED"
mkdir -p "$OUT"
run() { # dir arm extra...
  local d="$OUT/$1"; shift
  if [ ! -f "$d/summary.json" ]; then
    $BIN/life-run run --config "$CFG" --out "$d" --world-seed $SEED --ticks $TICKS "$@" > /dev/null || echo "FAILED $d"
  fi
}
for arm in full frozen readout homeo freeze_at fb_wt ltc; do
  run $arm --arm $arm
done
run full_shuffled --arm full --variant shuffled
run full_iid --arm full --variant iid
run full_perm --arm full --perm $((SEED + 5000))
# identical streams across arms (actions across variants)
$BIN/life-lab compare --runs "$OUT/full,$OUT/frozen,$OUT/readout,$OUT/homeo,$OUT/freeze_at,$OUT/fb_wt,$OUT/ltc" > "$OUT/compare_main.txt" || echo "COMPARE FAILED seed $SEED"
$BIN/life-lab compare --actions-only --runs "$OUT/full,$OUT/full_shuffled,$OUT/full_iid,$OUT/full_perm" > "$OUT/compare_variants.txt" || echo "COMPARE(variants) FAILED seed $SEED"
# re-adaptation branches at each structural change of the full life (within the horizon)
$BIN/life-lab changes --run "$OUT/full" > "$OUT/changes.txt"
while read -r tc sites; do
  [ -z "$tc" ] && continue
  if [ $((tc + 50000)) -le $TICKS ]; then
    for mode in fresh frozen; do
      d="$OUT/branch_${tc}_$mode"
      if [ ! -f "$d/summary.json" ]; then
        $BIN/life-run branch --run "$OUT/full" --at $tc --mode $mode --ticks 50000 --fresh-seed $((SEED * 31 + tc)) --out "$d" > /dev/null || echo "BRANCH FAILED $d"
      fi
    done
    $BIN/life-lab readapt --run "$OUT/full" --at $tc --fresh "$OUT/branch_${tc}_fresh" --frozen "$OUT/branch_${tc}_frozen" --out "$OUT/readapt_$tc.json" > /dev/null || echo "READAPT FAILED $tc"
  fi
done < "$OUT/changes.txt"
for arm in full frozen readout; do
  $BIN/life-lab audit --run "$OUT/$arm" >> "$OUT/audit.txt" || echo "AUDIT FAILED $arm"
  $BIN/life-lab summary --run "$OUT/$arm" --window 100000 > "$OUT/summary_$arm.txt"
done
$BIN/life-lab evaluate --root "$OUT" --seed $SEED > "$OUT/evaluate.txt" || echo "EVALUATE FAILED seed $SEED"
echo "seed $SEED done"
