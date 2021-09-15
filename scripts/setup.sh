#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

sudo apt update
sudo apt install -y git clang curl libssl-dev llvm libudev-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.bashrc

rustup toolchain add nightly-2021-07-03
rustup target add wasm32-unknown-unknown --toolchain nightly-2021-07-03