#!/usr/bin/env bash
# Run on macOS with OrbStack: poc/scripts/linux.sh [MACHINE_NAME]
# The default machine is rateq-ubuntu-arm64. The machine must exist, have Rust,
# Cargo, and a C toolchain installed, and be able to access this checkout at
# /mnt/mac. For Ubuntu setup, see the commands in local.sh. The script runs
# local.sh inside the machine; its log and output paths are described there.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
machine="${1:-rateq-ubuntu-arm64}"

exec orb -m "$machine" bash -l "/mnt/mac${script_dir}/local.sh"
