#!/usr/bin/env bash
#
# ci-local.sh — run what .github/workflows/ci.yml runs, locally, before pushing.
#
# For each crate: a warnings-denied build with all features, then the tests
# with all features. CI uses the latest stable toolchain: keep yours current
# (`rustup update stable`) or a new lint will only show up on GitHub.
#
# Usage:
#     scripts/ci-local.sh                    # all four crates, in CI's order
#     scripts/ci-local.sh rustf rustf-cli    # only these crates
#
set -euo pipefail

cd "$(dirname "$0")/.."

CRATES=("$@")
if [[ ${#CRATES[@]} -eq 0 ]]; then
  CRATES=(rustf-macros rustf-schema rustf rustf-cli)
fi

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n'  "$*"; }

bold "toolchain: $(rustc --version)"

FAILED=()
for crate in "${CRATES[@]}"; do
  bold "── $crate: build (warnings denied) ──"
  if ! (cd "$crate" && RUSTFLAGS="-D warnings" cargo build --all-features --quiet); then
    FAILED+=("$crate build")
    continue
  fi
  bold "── $crate: test ──"
  if ! (cd "$crate" && cargo test --all-features --no-fail-fast --quiet); then
    FAILED+=("$crate test")
  fi
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  red "✗ failed: ${FAILED[*]}"
  exit 1
fi
green "✓ all crates build without warnings and pass their tests"
