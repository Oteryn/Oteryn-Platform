#!/bin/sh
set -eu

decode_gateway_hex() {
    php -r '
        $hex = strtoupper($argv[1] ?? "");
        if (preg_match("/^[0-9A-F]{8}$/", $hex) !== 1) {
            exit(2);
        }

        $octets = array_map(
            static fn (string $byte): int => hexdec($byte),
            array_reverse(str_split($hex, 2)),
        );
        $address = implode(".", $octets);

        if (filter_var($address, FILTER_VALIDATE_IP, FILTER_FLAG_IPV4) === false) {
            exit(3);
        }

        echo $address;
    ' "$1"
}

is_private_ipv4() {
    php -r '
        $address = $argv[1] ?? "";
        if (filter_var($address, FILTER_VALIDATE_IP, FILTER_FLAG_IPV4) === false) {
            exit(2);
        }

        $parts = array_map("intval", explode(".", $address));
        $private = $parts[0] === 10
            || ($parts[0] === 172 && $parts[1] >= 16 && $parts[1] <= 31)
            || ($parts[0] === 192 && $parts[1] === 168);

        exit($private ? 0 : 3);
    ' "$1"
}

resolve_docker_gateway() {
    gateway_hex="$(awk '$2 == "00000000" { print $3; exit }' /proc/net/route)"
    if [ -z "$gateway_hex" ]; then
        echo "Unable to resolve the Docker default gateway." >&2
        return 1
    fi

    gateway="$(decode_gateway_hex "$gateway_hex")"
    if ! is_private_ipv4 "$gateway"; then
        echo "Docker default gateway is not an exact RFC1918 IPv4 address." >&2
        return 1
    fi

    printf '%s' "$gateway"
}

if [ "${1:-}" = "--self-test" ]; then
    test "$(decode_gateway_hex 010012AC)" = "172.18.0.1"
    is_private_ipv4 10.0.0.1
    is_private_ipv4 172.31.255.254
    is_private_ipv4 192.168.1.1
    if is_private_ipv4 203.0.113.1; then
        echo "Public IPv4 address unexpectedly passed private-gateway validation." >&2
        exit 4
    fi
    exit 0
fi

if [ -z "${TRUSTED_PROXIES:-}" ]; then
    TRUSTED_PROXIES="$(resolve_docker_gateway)"
    export TRUSTED_PROXIES
fi

if [ "$TRUSTED_PROXIES" = "*" ]; then
    echo "Wildcard reverse-proxy trust is forbidden." >&2
    exit 5
fi

# The Synology Platform origin is loopback-only and browser traffic terminates TLS
# at the owner-designated Cloudflare Tunnel, so session cookies must remain secure.
SESSION_SECURE_COOKIE=true
export SESSION_SECURE_COOKIE

exec "$@"
