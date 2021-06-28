name := "nft360"

build:
    cargo build --release

test crate:
    cargo test --package {{crate}} --lib

build-bench:
    cd node && cargo build --release --features runtime-benchmarks

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
        --template=./.maintain/frame-weight-template.hbs
