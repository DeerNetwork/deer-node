#!/bin/bash

# Runs all benchmarks for all pallets, for a given runtime, provided by $1
# Should be run on a reference machine to gain accurate benchmarks
# current reference machine: https://github.com/paritytech/substrate/pull/5848

standard_args="--release --locked --features=runtime-benchmarks"
pallets_file=deer-pallets

echo "[+] Running all benchmarks for $runtime"

# shellcheck disable=SC2086
cargo +nightly run $standard_args benchmark \
    --chain dev \
    --list |\
  tail -n+2 |\
  cut -d',' -f1 |\
  uniq | \
  grep -v frame_system > $pallets_file

# For each pallet found in the previous command, run benches on each function
while read -r line; do
  pallet="$(echo "$line" | cut -d' ' -f1)";
  echo "Pallet: $pallet";
# shellcheck disable=SC2086
cargo +nightly run $standard_args -- benchmark \
  --chain=dev \
  --steps=50 \
  --repeat=20 \
  --pallet="$pallet" \
  --extrinsic="*" \
  --execution=wasm \
  --wasm-execution=compiled \
  --heap-pages=4096 \
  --output="./node/runtime/src/weights/${pallet/::/_}.rs"
done < $pallets_file
rm $pallets_file
