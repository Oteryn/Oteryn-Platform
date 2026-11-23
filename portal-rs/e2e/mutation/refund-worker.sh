#!/usr/bin/env bash
set -euo pipefail

: "${PORTAL_E2E_REFUND_BINARY:?Set the Portal binary path}"
: "${PORTAL_E2E_REFUND_KEY_FILE:?Set the local sandbox signing key path}"
export OTERYN_SHOP_TEST_PAYMENT_KEY="$(cat "$PORTAL_E2E_REFUND_KEY_FILE")"
if [[ ${1:-} == worker ]]; then
  export OTERYN_WALLET_ENABLED=false
fi
exec "$PORTAL_E2E_REFUND_BINARY" "$@"
