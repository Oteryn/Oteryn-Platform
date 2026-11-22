#!/usr/bin/env bash
set -euo pipefail

PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --no-audit --no-fund
if [[ -z "${CHROMIUM_PATH:-}" ]]; then
  install_args=(chromium)
  if [[ "${GITHUB_ACTIONS:-}" == true ]]; then
    install_args+=(--with-deps)
  fi
  node node_modules/@playwright/test/cli.js install "${install_args[@]}"
  CHROMIUM_PATH="$(node --input-type=module -e 'import { chromium } from "@playwright/test"; console.log(chromium.executablePath())')"
fi
if [[ ! -x "$CHROMIUM_PATH" ]]; then
  echo 'CHROMIUM_PATH must point to an executable Chromium binary.' >&2
  exit 1
fi
export CHROMIUM_PATH
if (( $(cat /proc/sys/net/ipv4/ip_unprivileged_port_start) > 465 )); then
  sudo sysctl -w net.ipv4.ip_unprivileged_port_start=0
fi

export POSTGRES_PASSWORD
POSTGRES_PASSWORD="$(openssl rand -hex 32)"
container="$(docker run --rm -d -p 127.0.0.1::5432 -e POSTGRES_PASSWORD \
  postgres:18.6@sha256:e3d24f6906d0760bf4da0dae872415391ededefad23fd471c47dc53b0ddf3c42)"
trap 'docker stop "$container" >/dev/null' EXIT
address="$(docker port "$container" 5432/tcp)"
export PORTAL_E2E_ADMIN_DATABASE_URL="postgres://postgres:${POSTGRES_PASSWORD}@${address}/postgres"
unset POSTGRES_PASSWORD
node tiers/run.mjs "$@"
