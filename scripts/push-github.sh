#!/usr/bin/env bash
#
# push-github.sh — publish the repository to GitHub without the AI tooling.
#
# The internal repository holds everything; GitHub receives a filtered mirror
# of the same history with the paths below removed from every commit. The
# filtering is deterministic, so the same internal commits always produce the
# same public commits and ordinary pushes work after the first one.
#
# Usage:
#     scripts/push-github.sh                     # dry run: build + check the mirror, push nothing
#     scripts/push-github.sh --execute           # push branches and v* tags to the `github` remote
#     scripts/push-github.sh --execute --force   # same, allowing history to be rewritten on GitHub
#                                                # (needed once: the first filtered push)
#     BRANCHES="main" scripts/push-github.sh ... # choose branches (default: main dev)
#     GITHUB_URL=<url> scripts/push-github.sh ... # target another URL than the `github` remote
#
# Requirements:
#   - git-filter-repo (https://github.com/newren/git-filter-repo)
#   - a remote named `github` pointing at the public repository
#
# Only committed history is published: the working tree and the index are
# never read or modified.
#
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

# Paths that exist only in the internal repository.
PRIVATE_PATHS=(
  .claude/
  .wolf/
  graft/
  CLAUDE.md
  AGENTS.md
  .mcp.json
  opencode.json
  .ignore
)

BRANCHES="${BRANCHES:-main dev}"
WORK="$ROOT/tmp/github-mirror.git"

EXECUTE=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --execute) EXECUTE=1 ;;
    --dry-run) EXECUTE=0 ;;
    --force) FORCE=1 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n'  "$*"; }

# ── Preflight ────────────────────────────────────────────────────────────────

if ! command -v git-filter-repo >/dev/null 2>&1; then
  red "✗ git-filter-repo is not installed (brew install git-filter-repo)"
  exit 1
fi

# GITHUB_URL overrides the `github` remote (e.g. to test against a local
# bare repository).
if [[ -z "${GITHUB_URL:-}" ]] && ! GITHUB_URL="$(git remote get-url github 2>/dev/null)"; then
  red "✗ no 'github' remote. Add it with: git remote add github <url>"
  exit 1
fi

for branch in $BRANCHES; do
  if ! git rev-parse --verify --quiet "refs/heads/$branch" >/dev/null; then
    red "✗ local branch '$branch' does not exist"
    exit 1
  fi
done

# ── Build the filtered mirror ────────────────────────────────────────────────

bold "── Building filtered mirror in tmp/github-mirror.git ──"
rm -rf "$WORK"
mkdir -p "$(dirname "$WORK")"
# --no-local: a real clone, so filter-repo accepts it as a fresh one.
git clone --quiet --bare --no-local "$ROOT" "$WORK"

FILTER_ARGS=(--invert-paths --force --quiet)
for path in "${PRIVATE_PATHS[@]}"; do
  FILTER_ARGS+=(--path "$path")
done
git -C "$WORK" filter-repo "${FILTER_ARGS[@]}"

# ── Check: no private path in any published commit ──────────────────────────

REFS=()
for branch in $BRANCHES; do
  REFS+=("refs/heads/$branch")
done
TAGS=$(git -C "$WORK" tag --list 'v*')
for tag in $TAGS; do
  REFS+=("refs/tags/$tag")
done

# One anchored pattern per path: a directory matches everything under it,
# a file matches only itself.
PATTERN=""
for path in "${PRIVATE_PATHS[@]}"; do
  escaped=$(printf '%s' "$path" | sed 's/[.]/\\./g')
  if [[ "$path" == */ ]]; then
    PATTERN+="${PATTERN:+|}^${escaped}"
  else
    PATTERN+="${PATTERN:+|}^${escaped}\$"
  fi
done
LEAKS=$(git -C "$WORK" log --format= --name-only "${REFS[@]}" | sort -u | grep -E "$PATTERN" || true)
if [[ -n "$LEAKS" ]]; then
  red "✗ private paths still present in the mirror:"
  echo "$LEAKS"
  exit 1
fi
green "✓ no private path in the history of: $BRANCHES${TAGS:+ + tags}"

for branch in $BRANCHES; do
  printf '  %-6s internal %s  ->  public %s\n' "$branch" \
    "$(git rev-parse --short "refs/heads/$branch")" \
    "$(git -C "$WORK" rev-parse --short "refs/heads/$branch")"
done

# ── Push ─────────────────────────────────────────────────────────────────────

PUSH_ARGS=()
[[ "$FORCE" -eq 1 ]] && PUSH_ARGS+=(--force)
[[ "$EXECUTE" -eq 0 ]] && PUSH_ARGS+=(--dry-run)

if [[ "$EXECUTE" -eq 0 ]]; then
  bold "── DRY RUN (nothing is pushed; pass --execute to push) ──"
else
  bold "── PUSH to $GITHUB_URL ──"
fi

# `${a[@]+...}`: bash 3.2 treats an empty array as unset under `set -u`.
git -C "$WORK" push ${PUSH_ARGS[@]+"${PUSH_ARGS[@]}"} "$GITHUB_URL" "${REFS[@]}" || {
  red "✗ push refused."
  if [[ "$FORCE" -eq 0 ]]; then
    echo "  If GitHub still has the unfiltered history (first filtered push),"
    echo "  rerun with --force: it rewrites the published branches."
  fi
  exit 1
}

green "✓ done"
