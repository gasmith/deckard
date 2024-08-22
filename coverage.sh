#!/bin/bash

set -euo pipefail

git rev-parse --show-toplevel || echo .

function clean() {
  rm -f *.profraw
}
trap clean EXIT

CARGO_INCREMENTAL=0 \
  RUSTFLAGS='-Cinstrument-coverage' \
  LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' \
  cargo test

grcov . \
  --binary-path ./target/debug/deps/ \
  --source-dir . \
  --output-types html \
  --branch \
  --ignore-not-existing \
  --output-path target/coverage/html
