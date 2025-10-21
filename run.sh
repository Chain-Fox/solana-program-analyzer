#!/usr/bin/env bash

cargo build
# export RUSTC_WRAPPER=${PWD}/target/debug/easy-analysis
# export RUSTC=${PWD}/target/debug/easy-analysis
export RUST_BACKTRACE=full
export LOCKBUD_LOG=info
# export LD_LIBRARY_PATH="/home/chain-fox/.rustup/toolchains/nightly-2025-08-14-x86_64-unknown-linux-gnu/lib/":$LD_LIBRARY_PATH

BASE=${PWD}
cargo build  # build checker

echo $1

pushd $1
# cargo clean


export RUSTC=${BASE}/target/debug/solana-program-analyzer
# export RUST_BACKTRACE=full
# export LOCKBUD_LOG=info
export LD_LIBRARY_PATH="/home/chain-fox/.rustup/toolchains/nightly-2025-10-02-x86_64-unknown-linux-gnu/lib/":$LD_LIBRARY_PATH

# cargo build
RUSTC_FLAGS="-C overflow-checks=no"
cargo check

popd
