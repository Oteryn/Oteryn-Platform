#!/usr/bin/env bash
# One command for the cross-repository E2E harness (README.md, "Cross-repository harness").
# Needs PORTAL_E2E_ADMIN_DATABASE_URL (a throwaway LOCAL PostgreSQL 18.6 administrator URL),
# Node 22+, OpenSSL and the repository Rust toolchain. Arguments go to harness.mjs:
#   portal-rs/e2e/start.sh [--seed-plan plan.json] [--exit-after-ready]
set -euo pipefail
set +x
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "${PORTAL_E2E_ADMIN_DATABASE_URL:-}" ]; then
  echo "start.sh: set PORTAL_E2E_ADMIN_DATABASE_URL to a throwaway local PostgreSQL administrator URL" >&2
  exit 2
fi
node_major="$(node -p 'process.versions.node.split(".")[0]')"
if [ "$node_major" -lt 22 ]; then
  echo "start.sh: Node 22+ is required" >&2
  exit 2
fi
command -v openssl >/dev/null || { echo "start.sh: openssl is required" >&2; exit 2; }
if [ ! -d "$here/node_modules/pg" ]; then
  # Only the pinned lockfile; no browser download and no install scripts.
  (cd "$here" && PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --no-audit --no-fund >&2)
fi
exec node "$here/harness.mjs" "$@"
