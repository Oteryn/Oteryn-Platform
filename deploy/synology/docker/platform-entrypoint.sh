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

# Native admission signing key custody: the keys live in a root-only named volume mounted read-only
# at this path, written by deploy/synology/scripts/generate-native-admission-key.sh. Only the
# key id and file path are exported; the key bytes stay in the file and are never read here.
load_native_admission_keys() {
    key_dir=/run/oteryn-native-admission
    for role in current retiring; do
        [ -f "$key_dir/$role.kid" ] || continue
        key_id="$(head -c 65 "$key_dir/$role.kid" | tr -d '\n')"
        if ! printf '%s' "$key_id" | grep -Eq '^[A-Za-z0-9._-]{1,64}$'; then
            echo "Native admission $role key id file is invalid." >&2
            return 1
        fi
        if [ "$role" = current ]; then
            GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE="$key_dir/current.key"
            GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID="$key_id"
            export GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID
        else
            GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_FILE="$key_dir/retiring.key"
            GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_ID="$key_id"
            export GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_FILE GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_ID
        fi
    done
}

# Platform internal mTLS listener (README: Platform internal mTLS listener). With
# OTERYN_PLATFORM_FASTCGI_ENABLED=true the image's single FPM pool serves the Platform front controller
# on /run/oteryn-fastcgi/php-fpm.sock, a volume shared only with the internal-mtls terminator. It is
# started after the runtime environment above is exported (the pool keeps it, clear_env=no), runs next
# to the default command and must be listening before that command starts; otherwise the container
# exits (fail closed). While it runs, this shell stays PID 1 and supervises both (run_supervised): if
# either exits, the other is stopped and the container exits, so `restart: always` brings back both.
# Any value other than true or false is refused.
fastcgi_socket=/run/oteryn-fastcgi/php-fpm.sock
fastcgi_pid=
start_platform_fastcgi() {
    case "${OTERYN_PLATFORM_FASTCGI_ENABLED:-false}" in
        false) return 0 ;;
        true) ;;
        *)
            echo "OTERYN_PLATFORM_FASTCGI_ENABLED must be true or false." >&2
            return 1
            ;;
    esac
    # Only the container's main process owns the pool. The deploy scripts also run this entrypoint
    # through `docker exec` (trust publication, signing self-check); those must not start a second one.
    [ "$$" = 1 ] || return 0
    if ! command -v php-fpm >/dev/null 2>&1; then
        echo "Platform FastCGI is enabled but this image has no php-fpm." >&2
        return 1
    fi
    if [ ! -d "${fastcgi_socket%/*}" ] || [ ! -w "${fastcgi_socket%/*}" ]; then
        echo "Platform FastCGI socket directory is missing or not writable." >&2
        return 1
    fi
    # A socket left by a previous container on the shared volume must not satisfy the wait below.
    rm -f "$fastcgi_socket"
    php-fpm --nodaemonize &
    fastcgi_pid=$!
    attempts=0
    while [ ! -S "$fastcgi_socket" ]; do
        attempts=$((attempts + 1))
        if [ "$attempts" -gt 50 ] || ! kill -0 "$fastcgi_pid" 2>/dev/null; then
            echo "Platform FastCGI did not start listening." >&2
            return 1
        fi
        sleep 0.1
    done
}

# Runs the main command next to the FPM master. The first of the two to exit, or a stop signal, ends
# both; a pool that died while the main command keeps running would otherwise leave every mTLS
# request failing with 502 inside a container that still looks healthy. Liveness is polled from
# /proc (a dead, unreaped child is a zombie) because BusyBox `wait -n` misses children killed by a
# signal.
process_alive() {
    [ -r "/proc/$1/status" ] && ! grep -q '^State:[[:space:]]*Z' "/proc/$1/status" 2>/dev/null
}
stop_supervised() {
    kill -TERM "$main_pid" "$fastcgi_pid" 2>/dev/null || true
    attempts=0
    while { process_alive "$main_pid" || process_alive "$fastcgi_pid"; } && [ "$attempts" -lt 100 ]; do
        attempts=$((attempts + 1))
        sleep 0.1
    done
}
run_supervised() {
    "$@" &
    main_pid=$!
    trap 'stop_supervised; exit 143' TERM INT
    while process_alive "$main_pid" && process_alive "$fastcgi_pid"; do
        sleep 1 &
        wait "$!" 2>/dev/null || true
    done
    if process_alive "$fastcgi_pid"; then
        echo "Platform main process exited; stopping the FastCGI pool." >&2
        status=9
    else
        echo "Platform FastCGI pool exited; stopping the container so it is restarted." >&2
        status=8
    fi
    stop_supervised
    exit "$status"
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
    if OTERYN_PLATFORM_FASTCGI_ENABLED=yes start_platform_fastcgi 2>/dev/null; then
        echo "Invalid FastCGI switch unexpectedly passed." >&2
        exit 4
    fi
    OTERYN_PLATFORM_FASTCGI_ENABLED=false start_platform_fastcgi
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

load_native_admission_keys || exit 6
start_platform_fastcgi || exit 7

if [ -n "$fastcgi_pid" ]; then
    run_supervised "$@"
fi
exec "$@"
