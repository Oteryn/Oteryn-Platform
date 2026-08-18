#!/usr/bin/env bash
set -euo pipefail

package_name="${1:-}"
repository_owner="${OTERYN_GHCR_OWNER:-${GITHUB_REPOSITORY_OWNER:-}}"
repository_owner="${repository_owner,,}"

if [[ ! "$repository_owner" =~ ^[a-z0-9][a-z0-9-]{0,38}$ ]]; then
    echo "Repository owner must resolve to a bounded lowercase GitHub owner for GHCR." >&2
    exit 1
fi
if [[ ! "$package_name" =~ ^[a-z0-9][a-z0-9._-]{0,127}$ ]]; then
    echo "Package name must be a bounded lowercase GHCR package name." >&2
    exit 1
fi

printf 'ghcr.io/%s/%s\n' "$repository_owner" "$package_name"
