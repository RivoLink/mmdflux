#!/usr/bin/env bash
#
# Trigger the Release Plan workflow, wait for it to complete,
# download the report, and print it to stdout.
#
# Usage:
#   ./scripts/run-release-plan.sh          # plan mode (default)
#   ./scripts/run-release-plan.sh release  # release mode
#   ./scripts/run-release-plan.sh release mmdflux  # release a specific package
#
set -euo pipefail

MODE="${1:-plan}"
PACKAGE="${2:-}"
REPO="kevinswiber/mmdflux"
WORKFLOW="release-plan.yml"
TMPDIR=$(mktemp -d)

trap 'rm -rf "$TMPDIR"' EXIT

# Build dispatch flags
flags=(-f "mode=${MODE}")
if [ -n "$PACKAGE" ]; then
  flags+=(-f "package=${PACKAGE}")
fi

echo "Dispatching Release Plan workflow (mode=${MODE})..."
gh workflow run "$WORKFLOW" --repo "$REPO" "${flags[@]}"

# Wait for the run to appear (dispatch is async)
sleep 3

# Find the run we just triggered
RUN_ID=$(gh run list --repo "$REPO" --workflow "$WORKFLOW" --limit 1 \
  --json databaseId --jq '.[0].databaseId')

if [ -z "$RUN_ID" ]; then
  echo "error: could not find workflow run" >&2
  exit 1
fi

echo "Waiting for run ${RUN_ID}..."
gh run watch "$RUN_ID" --repo "$REPO" --exit-status || {
  echo "error: workflow run failed" >&2
  gh run view "$RUN_ID" --repo "$REPO"
  exit 1
}

echo ""

# Download and display the report
gh run download "$RUN_ID" --repo "$REPO" --name release-plan --dir "$TMPDIR"

if [ -f "$TMPDIR/release-plan.md" ]; then
  echo "To re-download: gh run download ${RUN_ID} -n release-plan"
  echo ""
  if command -v bat &>/dev/null; then
    bat --paging=never "$TMPDIR/release-plan.md"
  else
    cat "$TMPDIR/release-plan.md"
  fi
else
  echo "error: release-plan.md not found in artifacts" >&2
  exit 1
fi
