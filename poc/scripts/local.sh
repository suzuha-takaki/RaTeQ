#!/usr/bin/env bash
# Run from any directory on macOS or Linux: poc/scripts/local.sh
# Requires Rust/Cargo and a C build toolchain. On Ubuntu, install the latter with:
#   sudo apt-get update && sudo apt-get install -y build-essential
# Cargo may need network access to download the locked dependencies on the first run.
# Each run creates a timestamped log under poc/logs and PNG/SVG samples
# under poc/output. The output directory is ignored by Git. Build artifacts
# go to ~/.cache/rateq-poc/<OS>-<architecture> to keep platforms separate.
# Timings exclude PNG decoding, protocol construction, and file writes. Both
# passes share a process and overwrite the case images; later cases use warm caches.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
poc_dir="$(cd -- "$script_dir/.." && pwd -P)"
platform="$(uname -s)-$(uname -m)"

mkdir -p "$poc_dir/logs" "$poc_dir/output"
output_dir="$(mktemp -d "$poc_dir/output/$(date +%Y%m%d_%H%M%S)_${platform}_XXXXXX")"
log_file="$poc_dir/logs/$(basename -- "$output_dir").log"
export CARGO_TARGET_DIR="$HOME/.cache/rateq-poc/$platform"

printf 'Log: %s\nOutput: %s\n' "$log_file" "$output_dir"

{
    uname -sm
    rustc --version
    cargo --version
    cargo run --locked --release \
        --manifest-path "$poc_dir/Cargo.toml" \
        -- "$output_dir"
} 2>&1 | tee "$log_file"
