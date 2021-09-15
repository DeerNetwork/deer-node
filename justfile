name := "deer-node"

build +args='--release':
    cargo build {{args}}

test crate +args='--lib':
    cargo test --package {{crate}} {{args}}

bench pallet:
    #!/bin/bash
    cargo build --release --locked --features=runtime-benchmarks
    ./target/release/{{name}} benchmark \
    --chain=dev \
    --steps=50 \
    --repeat=20 \
    --pallet=pallet-{{pallet}} \
    --extrinsic=* \
    --execution=wasm \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output=./pallets/{{pallet}}/src/weights.rs \
    --template=./scripts/frame-weight-template.hbs


run +args='--dev --tmp':
    ./target/release/{{name}} \
    {{args}} \
    --port 30333 \
    --ws-port 9944 \
    --rpc-port 9933 \
    --rpc-methods Unsafe \
    --unsafe-rpc-external \
    --rpc-cors all \
    --unsafe-ws-external

try-runtime block *args:
    cargo run \
    --features=try-runtime \
    try-runtime \
    --block-at {{block}} \
    {{args}} \
    on-runtime-upgrade \
    live
