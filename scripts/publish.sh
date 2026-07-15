#!/usr/bin/env bash
#
# publish.sh — publish the RustF crates to crates.io in dependency order.
#
# Crates have no workspace manifest and depend on each other by path+version,
# so each must be live on the index before the crate that depends on it is
# published. Order:
#
#     rustf-macros ─┐
#     rustf-schema ─┴─► rustf ─► rustf-cli
#
# Usage:
#     scripts/publish.sh            # dry-run (default, publishes nothing)
#     scripts/publish.sh --execute  # actually publish to crates.io
#
# Requirements:
#   - `cargo login <token>` already done (token needs publish-new + publish-update)
#   - clean git working tree
#   - all four Cargo.toml versions identical
#
set -euo pipefail

cd "$(dirname "$0")/.."

# Publish order — leaves first, dependents last.
CRATES=(rustf-macros rustf-schema rustf rustf-cli)

# Team that should co-own every crate (so the org can publish updates). You stay
# a named admin owner; this just adds the team as publisher. Override with
# RUSTF_PUBLISH_OWNER, or set it empty to skip owner management entirely.
# Adding owners needs a token with the `change-owners` scope, and the
# github:numerum-tech:rustf team must exist with you as a member.
OWNER="${RUSTF_PUBLISH_OWNER:-github:numerum-tech:rustf}"

EXECUTE=0
for arg in "$@"; do
  case "$arg" in
    --execute) EXECUTE=1 ;;
    --dry-run) EXECUTE=0 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n'  "$*"; }

# ── Preflight ────────────────────────────────────────────────────────────────

# 1. clean git tree
if [[ -n "$(git status --porcelain)" ]]; then
  red "✗ working tree is dirty — commit or stash before publishing."
  git status --short
  exit 1
fi

# 2. all versions match
VERSION=""
for c in "${CRATES[@]}"; do
  v=$(grep -m1 '^version' "$c/Cargo.toml" | cut -d'"' -f2)
  if [[ -z "$VERSION" ]]; then
    VERSION="$v"
  elif [[ "$v" != "$VERSION" ]]; then
    red "✗ version mismatch: $c is $v, expected $VERSION"
    exit 1
  fi
done
green "✓ clean tree, all crates at version $VERSION"

if [[ -n "$OWNER" ]]; then
  echo "  owner to add after each publish: $OWNER"
else
  echo "  owner management: disabled (RUSTF_PUBLISH_OWNER empty)"
fi

if [[ "$EXECUTE" -eq 0 ]]; then
  bold "── DRY RUN (no crate will be published; pass --execute to publish) ──"
else
  bold "── LIVE PUBLISH to crates.io at version $VERSION ──"
  read -r -p "Type the version to confirm: " confirm
  if [[ "$confirm" != "$VERSION" ]]; then
    red "✗ confirmation mismatch — aborting."
    exit 1
  fi
fi

# ── Wait for a crate version to appear on the index ──────────────────────────
# cargo publish already blocks until the new version is in the index, but we
# double-check before publishing a dependent so a slow index never breaks the
# chain.
wait_for_index() {
  local name="$1" want="$2"
  local url="https://crates.io/api/v1/crates/${name}/${want}"
  for _ in $(seq 1 60); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      green "  ✓ ${name}@${want} live on crates.io"
      return 0
    fi
    sleep 5
  done
  red "  ✗ timed out waiting for ${name}@${want} on the index"
  return 1
}

# ── Publish loop ─────────────────────────────────────────────────────────────
for c in "${CRATES[@]}"; do
  bold "→ $c"
  if [[ "$EXECUTE" -eq 0 ]]; then
    ( cd "$c" && cargo publish --dry-run )
  else
    ( cd "$c" && cargo publish )
    # Add the org team as co-owner (idempotent: a re-run just reports it's
    # already an owner, which we don't treat as fatal).
    if [[ -n "$OWNER" ]]; then
      if ( cd "$c" && cargo owner --add "$OWNER" ); then
        green "  ✓ added owner $OWNER to $c"
      else
        red "  ! could not add owner $OWNER to $c (already owner? missing change-owners scope? team missing?) — add manually later"
      fi
    fi
    # Don't gate the final crate — nothing depends on it.
    if [[ "$c" != "rustf-cli" ]]; then
      wait_for_index "$c" "$VERSION"
    fi
  fi
done

green "✓ done."
if [[ "$EXECUTE" -eq 0 ]]; then
  echo "Dry run only. Re-run with --execute to publish for real."
fi
