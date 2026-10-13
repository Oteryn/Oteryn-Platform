#!/usr/bin/env bash
# Cutover hook OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND (plan P6 step 5). cutover-postgres.sh native-smoke runs it,
# with no arguments, after the deploy turned native admission on and right after the Platform signing
# self-check (game-auth:native-admission:self-check) passed. It checks the native login path end to end:
#   - the deployed environment and both running containers have native admission on, paired
#     (Platform GAME_AUTH_NATIVE_ADMISSION_ENABLED and Gateway GATEWAY_NATIVE_LOGIN_ENABLED both true);
#   - a well-formed native (protocol_version 2) login with an unknown ticket, sent to the Gateway over host
#     loopback as health-check.sh does, reaches the Platform issuer and is rejected with 401
#     NATIVE_LOGIN_AUTHENTICATION_REQUIRED (not 400 refused at the Gateway, not 503 issuer unavailable);
#   - the read-only trust observation (D2) shows the first publication landed on the PostgreSQL store:
#     binding match, a profile present and not revoked, a non-null floor, and the configured key trusted.
# Read-only. Output: one result line; never an environment value, a ticket or a key.
set -euo pipefail
umask 077

HOOK_NAME=native-smoke
# shellcheck source=deploy/synology/scripts/cutover-hooks/common.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

(($# == 0)) || hook_fail "takes no arguments."
hook_require_tools docker jq python3 sed
hook_workdir
hook_runtime_env
[[ "$(hook_env_get OTERYN_NATIVE_ADMISSION_ENABLED)" == true ]] || hook_fail "native admission is not on in the deployed environment."

# container_flag <service> <key>: one environment value of the running service container.
container_flag() {
    local id
    id="$(hook_compose ps -q "$1" | head -n 1)"
    [[ -n "$id" ]] || hook_fail "the $1 service is not running."
    docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$id" \
        | awk -v prefix="$2=" 'index($0, prefix) == 1 && !found { print substr($0, length(prefix) + 1); found = 1 }'
}
platform_flag="$(container_flag platform GAME_AUTH_NATIVE_ADMISSION_ENABLED)"
gateway_flag="$(container_flag gateway GATEWAY_NATIVE_LOGIN_ENABLED)"
[[ "$platform_flag" == true && "$gateway_flag" == true ]] \
    || hook_fail "native admission is not on and paired in the running containers (platform=${platform_flag:-unset}, gateway=${gateway_flag:-unset})."

hook_loopback_python "$(hook_gateway_port)" <<'PY' || hook_fail "the native login smoke failed."
import http.client
import json
import secrets
import sys
import time

gateway_port = int(sys.argv[1])


def uuid7():
    value = (int(time.time() * 1000) & ((1 << 48) - 1)) << 80
    value |= 0x7 << 76
    value |= secrets.randbits(12) << 64
    value |= 0b10 << 62
    value |= secrets.randbits(62)
    text = f'{value:032x}'
    return f'{text[:8]}-{text[8:12]}-{text[12:16]}-{text[16:20]}-{text[20:]}'


def post(path, body):
    last_error = None
    for attempt in range(15):
        connection = http.client.HTTPConnection('127.0.0.1', gateway_port, timeout=5)
        try:
            connection.request('POST', path, body=body, headers={
                'Host': 'gateway.molehill.cloud',
                'Accept': 'application/json',
                'Content-Type': 'application/json',
            })
            response = connection.getresponse()
            return response.status, response.read(8192)
        except OSError as exc:
            last_error = exc
            if attempt == 14:
                break
            time.sleep(2)
        finally:
            connection.close()
    raise SystemExit(f'Host-loopback request failed after bounded retries: POST {path} ({type(last_error).__name__})')


native_login = json.dumps(
    {
        'protocol_version': 2,
        'game_login_ticket': 'oteryn-cutover-probe-' + secrets.token_hex(16),
        'attempt_ref': uuid7(),
        'character_id': uuid7(),
        'channel_id': None,
        'offer': {
            'client_build': 'cutover-probe',
            'client_platform': 'windows',
            'transports': [{'protocol_major': 1, 'transport_profile': 1, 'alpn': 'oteryn-game/1'}],
        },
    },
    separators=(',', ':'),
).encode()
status, body = post('/v1/login', native_login)
try:
    payload = json.loads(body)
except json.JSONDecodeError as exc:
    raise SystemExit(f'Native Gateway login did not return JSON (status {status})') from exc
error = payload.get('error') if isinstance(payload, dict) else None
code = error.get('code') if isinstance(error, dict) else None
if status != 401 or code != 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED':
    raise SystemExit(f'Native login did not reach the Platform issuer: status {status}, code {code}')
print('Verified native login through the Gateway to the Platform issuer (unknown ticket rejected as 401).')
PY

hook_observe "$HOOK_WORK/trust.json"
jq -e '
    .store.binding == "match"
    and (.profile | type == "object" and .present == true and .revoked != true)
    and (.floor | type == "object" and .value != null)
    and .key.trusted == true
    and .key.configured_key_id != null
    and .key.requested == .key.configured_key_id
' "$HOOK_WORK/trust.json" >/dev/null \
    || hook_fail "the trust observation does not show the first publication on the PostgreSQL store (binding match, profile, floor, trusted configured key)."

printf 'native_smoke_result=passed\n'
