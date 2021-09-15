#!/bin/bash

pallets_file=deer-pallets
node=./target/release/deer-node

cargo build --release --locked --features=runtime-benchmarks

$node benchmark --chain dev --list |\
  tail -n+2 |\
  cut -d',' -f1 |\
  uniq |\
  grep -v frame_system > $pallets_file

# For each pallet found in the previous command, run benches on each function
while read -r line; do
  pallet="$(echo "$line" | cut -d' ' -f1)";
  echo "Pallet: $pallet";
$node benchmark \
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
