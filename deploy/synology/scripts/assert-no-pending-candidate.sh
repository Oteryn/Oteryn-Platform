#!/usr/bin/env bash
set -euo pipefail
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
if [[ -f "$state_dir/candidate-release.env" ]]; then
    echo "Pending candidate metadata exists; refusing a new deployment transition." >&2
    exit 1
fi
