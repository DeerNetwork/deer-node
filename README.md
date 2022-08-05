# Deer Network

## Native Build

### Setup

Sync submodule

```
git submodule update --init
```

Setup rust enviroment and substrate dependencies

```
bash ./scripts/setup.sh
```

### Build

```
cargo build --release
```

### Run

Use Rust's native `cargo` command to build and launch the template node:

```sh
./target/release/deer-node --dev --tmp
```

## External Resources

- [deer-wiki](https://deernetwork.org/#/wiki/en/gettingStarted): The technical documentation.
- [deer-validator](https://github.com/DeerNetwork/deer-validator): Validator setup scripts for Deer network
- [deer-storage-mining](https://github.com/DeerNetwork/deer-storage-mining): Storage mining scripts for Deer Network