#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == 'serve' ]]; then
    export OTERYN_ACCOUNT_MFA=false
fi
exec "$(dirname -- "$0")/oteryn-portal" "$@"
