#!/bin/sh
set -euo pipefail

cargo build --release
target_dir="$PWD/target/release"
(cd ../.. && mix release --overwrite --path=${target_dir}/rel)
(cd $target_dir && ./demo)
