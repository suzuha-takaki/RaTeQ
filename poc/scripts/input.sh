#!/usr/bin/env bash
# Run interactively in the terminal being evaluated: poc/scripts/input.sh
# Requires the Rust/Cargo and build prerequisites described in local.sh.
# Press F2 to toggle mouse capture, F3 for bracketed paste, and F10 to quit.
# Try the keys, native selection/copy, and multiline paste. The screen shows
# recent crossterm events; record terminal/version, toggles and observations.
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
poc_dir="$(cd -- "$script_dir/.." && pwd -P)"
export CARGO_TARGET_DIR="$HOME/.cache/rateq-poc/$(uname -s)-$(uname -m)"
exec cargo run --locked --release --manifest-path "$poc_dir/Cargo.toml" -- input
