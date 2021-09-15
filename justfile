name := "deer-node"

build:
    cargo build --release

test crate +args='--lib':
    cargo test --package {{crate}} {{args}}

build-bench:
    cd node/cli && cargo build --release --features runtime-benchmarks

bench crate:
    #!/bin/bash
    ./target/release/{{name}} \
        benchmark \
        --chain=dev \
        --steps=50 \
        --repeat=20 \
        --pallet=pallet-{{crate}} \
        --extrinsic=* \
        --execution=wasm \
        --wasm-execution=compiled \
        --heap-pages=4096 \
        --output=./pallets/{{crate}}/src/weights.rs \
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
