#!/usr/bin/env bash

load_oteryn_env_file() {
    local env_file="$1"
    local line key value

    if [[ ! -f "$env_file" ]]; then
        echo "Missing staging environment file: $env_file" >&2
        return 1
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        if [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]]; then
            continue
        fi
        if [[ "$line" != *=* ]]; then
            echo "Invalid staging environment line; expected KEY=VALUE." >&2
            return 1
        fi

        key="${line%%=*}"
        value="${line#*=}"
        if [[ ! "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
            echo "Invalid staging environment key: $key" >&2
            return 1
        fi

        printf -v "$key" '%s' "$value"
        export "$key"
    done < "$env_file"

    # Character Bazaar Staging Control loads both a partial durable state file
    # and the complete ephemeral deployment .env. Apply the public-origin
    # migration only to the complete .env; the state file intentionally carries
    # Marketplace keys only and must not be treated as deployment configuration.
    if [[ "${GITHUB_WORKFLOW:-}" == "Character Bazaar Staging Control" \
        && "$(basename -- "$env_file")" == ".env" ]]; then
        case "${APP_URL:-}" in
            https://oteryn.molehill.cloud|http://127.0.0.1:8000) ;;
            *)
                echo "Character Bazaar public staging APP_URL must be the canonical origin or the exact legacy loopback value." >&2
                return 1
                ;;
        esac
        case "${SESSION_SECURE_COOKIE:-}" in
            true|false|'') ;;
            *)
                echo "Character Bazaar public staging SESSION_SECURE_COOKIE must be boolean." >&2
                return 1
                ;;
        esac

        APP_URL=https://oteryn.molehill.cloud
        SESSION_SECURE_COOKIE=true
        export APP_URL SESSION_SECURE_COOKIE
    fi
}
