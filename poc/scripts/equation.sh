#!/usr/bin/env bash
# Named-equation experiment. Requires Rust/Cargo and the toolchain in local.sh.
# Examples:
#   poc/scripts/equation.sh -i matrix -i large -i fraction --export
#   poc/scripts/equation.sh -i matrix -i large -i fraction --preview scroll
#   poc/scripts/equation.sh -i matrix -i large -i fraction --preview break
# Export writes a new directory under poc/output. Preview requires an interactive
# terminal. Use n/p to switch equations, arrows to scroll, and q/Esc to quit.
# Break mode only inserts preview-only breaks at top-level + operators; nested
# expressions and unbreakable oversized terms require scrolling. It is a PoC,
# not a general-purpose TeX line-breaking algorithm. Exports are never wrapped.
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
poc_dir="$(cd -- "$script_dir/.." && pwd -P)"
export CARGO_TARGET_DIR="$HOME/.cache/rateq-poc/$(uname -s)-$(uname -m)"
exec cargo run --locked --release --manifest-path "$poc_dir/Cargo.toml" -- equation "$@"
