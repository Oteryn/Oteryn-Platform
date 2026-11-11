#!/usr/bin/env bash
set -euo pipefail

address="${1:-}"
policy="${2:-}"

fail() {
    echo "$1" >&2
    exit 1
}

if [[ "$policy" != "loopback" && "$policy" != "private-or-loopback" ]]; then
    fail "unsupported IPv4 policy"
fi

IFS='.' read -r a b c d extra <<< "$address"
if [[ -n "${extra:-}" || -z "${a:-}" || -z "${b:-}" || -z "${c:-}" || -z "${d:-}" ]]; then
    fail "address must contain exactly four IPv4 octets"
fi

for octet in "$a" "$b" "$c" "$d"; do
    if [[ ! "$octet" =~ ^(0|[1-9][0-9]{0,2})$ ]]; then
        fail "IPv4 octets must be canonical decimal values"
    fi
    if (( 10#$octet > 255 )); then
        fail "IPv4 octet exceeds 255"
    fi
done

a_n=$((10#$a))
b_n=$((10#$b))

if [[ "$policy" == "loopback" ]]; then
    [[ "$address" == "127.0.0.1" ]] || fail "service must remain bound to exact loopback 127.0.0.1"
    exit 0
fi

if (( a_n == 127 )); then
    exit 0
fi
if (( a_n == 10 )); then
    exit 0
fi
if (( a_n == 172 && b_n >= 16 && b_n <= 31 )); then
    exit 0
fi
if (( a_n == 192 && b_n == 168 )); then
    exit 0
fi

fail "game bind must be loopback or an RFC1918 private IPv4 address"
