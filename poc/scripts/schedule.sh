#!/usr/bin/env bash
# Run on macOS or Linux: poc/scripts/schedule.sh
# Requires the Rust/Cargo and build prerequisites described in local.sh.
# Runs a deterministic rapid-edit burst using actual renderer child processes.
# Each child starts cold: process startup is included, and this is not an
# interactive responsiveness benchmark or an in-process cancellation API.
# The experiment reports stale results discarded and publishes the latest only.
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
poc_dir="$(cd -- "$script_dir/.." && pwd -P)"
export CARGO_TARGET_DIR="$HOME/.cache/rateq-poc/$(uname -s)-$(uname -m)"
exec cargo run --locked --release --manifest-path "$poc_dir/Cargo.toml" -- schedule
