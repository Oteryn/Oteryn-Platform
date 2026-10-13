#!/usr/bin/env bash
# Cutover hook OTERYN_CUTOVER_NATIVE_SMOKE_COMMAND (plan P6 step 5). cutover-postgres.sh native-smoke runs it,
# with no arguments, after the deploy turned native admission on and right after the Platform signing
# self-check (game-auth:native-admission:self-check) passed. It:
#   - checks that the deployed environment and both running containers have native admission on, paired
#     (Platform GAME_AUTH_NATIVE_ADMISSION_ENABLED and Gateway GATEWAY_NATIVE_LOGIN_ENABLED both true);
#   - closes the step-4 attempt evidence (PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md D5 step4-observe): after
#     the D3 anchor retry, two read-only observations at least 5 s apart (retried while the witness is busy or
#     a transaction is in flight), an anchored attempt-observation with the tool's D6 classification, and for
#     `committed` an anchored self-check record from a rerun of the read-only signing self-check and an
#     anchored run-close with done: true. A committed attempt still waiting for a passing self-check gets
#     D5 step4-recheck instead; an attempt already closed with done: true is left as it is. Any other
#     classification writes run-close with done: false (failed-before-witness leaves the run open for
#     provenance-init.sh retry, the evidence-retry step) and fails;
#   - a native login, actor to result, over host loopback with the canonical proxy headers (as health-check.sh):
#     a well-formed protocol_version 2 login with an unknown ticket must get 401 NATIVE_LOGIN_AUTHENTICATION_
#     REQUIRED from the Platform issuer; then the owner-provisioned native smoke actor signs in on the web,
#     authorizes the first-party native (Rust) OAuth client with PKCE (game:ticket), exchanges the code for a
#     bearer token, issues a native Game Login Ticket with it (after which the same bearer is refused: the
#     ticket issuance revoked it), and logs in through the Gateway with that ticket and its character: the
#     login must be admitted (200, the same attempt_ref, a World and channel, a Registry endpoint and an
#     oteryn-pre-admission-v1 grant), which covers ticket redemption, the scope assignment, route resolution and
#     grant signing on PostgreSQL; a replay of the same attempt must return the same admission. The web session
#     is signed out at the end. A 503 (route, reconciliation or issuer unavailable) is retried with the same
#     attempt a bounded number of times, as the contract allows; any other answer fails with its public code.
#     The actor comes from OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL, OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD (a secret,
#     passed to the request container by name only) and OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID: an identity
#     without MFA that owns that bootstrapped character on the published native route of a ready Game node
#     (NATIVE_LOGIN_OWNER_STEPS.md steps 9 and 10); the hook then prints native_smoke_login=end-to-end. A
#     missing, partly set or malformed actor fails closed before any evidence write, unless the owner waived the
#     successful native login for this cutover: OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER (workflow input
#     native_smoke_waiver) equal to the task id, with no actor set. Then only the unknown-ticket refusal is checked
#     and the hook prints native_smoke_login=owner-waived (stated limit: no successful end-to-end native login
#     before MariaDB retirement). A waiver that names another task, or a waiver next to an actor, is refused.
#     These logins write the OAuth, ticket and admission-attempt rows (P6 step 5 allows
#     witnessed writes);
#   - requires a fresh read-only trust observation (D2), taken after the login smoke with the attempt's key id,
#     to continue the committed attempt (the tool's D5 recheck predicate: same PostgreSQL identity and store,
#     a WAL position at or past every recorded observation, nothing in flight, and no blocking state) and to
#     show the first publication: binding match, a profile present and not revoked, a non-null floor, and the
#     configured key trusted. A rerun after the run was closed (an attempt already done: true) also takes this
#     check before the login smoke, so recorded evidence alone never passes.
# It acts only on a run whose run-open records cutover_ref "<task_id>/close-<close_run_id>" from
# OTERYN_CUTOVER_TASK_ID and OTERYN_CUTOVER_CLOSE_RUN_ID (the cutover step environment, which cutover-postgres.sh
# checks against its state); any other selected run is refused before a write.
# It changes no trust state; it writes only attempt evidence and its anchors. Inputs as provenance-init.sh:
# OTERYN_CUTOVER_ANCHOR_ISSUE, GITHUB_REPOSITORY and GH_TOKEN (issues: write). Output: key=value result lines
# only; never an environment value, a ticket, a key or an observation.
set -euo pipefail
umask 077

HOOK_NAME=native-smoke
HOOK_SELF="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=deploy/synology/scripts/cutover-hooks/common.sh
source "$HOOK_SELF/common.sh"
# shellcheck source=deploy/synology/scripts/cutover-hooks/evidence.sh
source "$HOOK_SELF/evidence.sh"

(($# == 0)) || hook_fail "takes no arguments."
ev_bind_cutover "${OTERYN_CUTOVER_TASK_ID:-}" "${OTERYN_CUTOVER_CLOSE_RUN_ID:-}"
hook_require_tools docker gh jq python3 sed
hook_evidence_setup
[[ -f "$EVIDENCE_TOOL" ]] || hook_fail "the attempt evidence tool is missing."
[[ -d "$EVIDENCE_DIR" ]] || hook_fail "the attempt evidence directory is missing."
hook_workdir
hook_runtime_env
[[ "$(hook_env_get OTERYN_NATIVE_ADMISSION_ENABLED)" == true ]] || hook_fail "native admission is not on in the deployed environment."
# The native smoke actor (owner-provisioned; never printed), or the owner's task-bound waiver of the successful
# native login. Checked before any evidence write; neither is a failure.
smoke_character="${OTERYN_CUTOVER_NATIVE_SMOKE_CHARACTER_ID:-}"
smoke_waiver="${OTERYN_CUTOVER_NATIVE_SMOKE_WAIVER:-}"
if [[ -n "$smoke_waiver" ]]; then
    [[ "$smoke_waiver" == "${OTERYN_CUTOVER_TASK_ID:-}" ]] \
        || hook_fail "the native smoke waiver (workflow input native_smoke_waiver) must equal this cutover's task id."
    [[ -z "${OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL:-}${OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD:-}$smoke_character" ]] \
        || hook_fail "both a native smoke waiver and a native smoke actor are set; give the actor or the waiver, not both."
    native_login=owner-waived
else
    native_login=end-to-end
    [[ "${OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL:-}" =~ ^[^[:space:]@]+@[^[:space:]@]+$ \
        && -n "${OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD:-}" ]] \
        || hook_fail "the native smoke actor is not configured: set OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_EMAIL (variable), OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_PASSWORD (secret) and OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_CHARACTER_ID (variable) for an identity without MFA that owns the smoke character, or, only on the owner's decision, dispatch with native_smoke_waiver set to the task id."
    [[ "$smoke_character" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]] \
        || hook_fail "OTERYN_STAGING_CUTOVER_NATIVE_SMOKE_CHARACTER_ID must be the smoke character's lower-case UUIDv7."
fi

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

ev_step4_after_deploy

client_id=-
if [[ "$native_login" == end-to-end ]]; then
    # The first-party native (Rust) OAuth client the Windows client uses; the deploy ensured it, so this only reads
    # and checks it, as the deploy and bootstrap-postgres.sh do.
    client_output="$(hook_artisan game-auth:oauth-client:ensure --rust 2>&1)" || hook_fail "the native OAuth client check failed."
    client_id="$(sed -n 's/.*OAuth client id: \([A-Za-z0-9-]*\).*/\1/p' <<<"$client_output" | tail -n 1)"
    [[ "$client_id" =~ ^[A-Za-z0-9-]{1,64}$ ]] || hook_fail "the native OAuth client check printed no client id."
    HOOK_LOOPBACK_ENV="OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD"
fi

hook_loopback_python "$(hook_platform_port)" "$(hook_gateway_port)" "$client_id" "${smoke_character:--}" "$native_login" \
    <<'PY' || hook_fail "the native login smoke failed."
import base64
import hashlib
import html
import http.client
import json
import os
import re
import secrets
import sys
import time
import urllib.parse

platform_port, gateway_port = int(sys.argv[1]), int(sys.argv[2])
client_id, character_id, native_login = sys.argv[3], sys.argv[4], sys.argv[5]
CANONICAL = 'oteryn.molehill.cloud'
REDIRECT_URI = 'http://127.0.0.1/callback'
PROXY_HEADERS = {
    'Host': CANONICAL,
    'X-Forwarded-Host': CANONICAL,
    'X-Forwarded-Proto': 'https',
    'X-Forwarded-Port': '443',
}


def uuid7():
    value = (int(time.time() * 1000) & ((1 << 48) - 1)) << 80
    value |= 0x7 << 76
    value |= secrets.randbits(12) << 64
    value |= 0b10 << 62
    value |= secrets.randbits(62)
    text = f'{value:032x}'
    return f'{text[:8]}-{text[8:12]}-{text[12:16]}-{text[16:20]}-{text[20:]}'


def exchange(port, method, path, body=None, headers=None, retries=1):
    """One request over host loopback; connection errors are retried (bounded), never an HTTP answer."""
    last_error = None
    for attempt in range(retries):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=10)
        try:
            connection.request(method, path, body=body, headers=headers or {})
            response = connection.getresponse()
            return response.status, response.getheaders(), response.read(1048576)
        except OSError as exc:
            last_error = exc
            if attempt + 1 < retries:
                time.sleep(2)
        finally:
            connection.close()
    raise SystemExit(f'Host-loopback request failed: {method} {path.split("?")[0]} ({type(last_error).__name__})')


def header(headers, name):
    return [value for key, value in headers if key.lower() == name]


def json_body(body, what):
    try:
        payload = json.loads(body)
    except ValueError as exc:
        raise SystemExit(f'{what} did not return JSON') from exc
    if not isinstance(payload, dict):
        raise SystemExit(f'{what} did not return a JSON object')
    return payload


def gateway_login(payload):
    status, _, body = exchange(gateway_port, 'POST', '/v1/login', json.dumps(payload, separators=(',', ':')).encode(), {
        'Host': 'gateway.molehill.cloud', 'Accept': 'application/json', 'Content-Type': 'application/json'}, retries=15)
    answer = json_body(body, f'Native Gateway login (status {status})')
    error = answer.get('error')
    return status, answer, (error.get('code') if isinstance(error, dict) else None)


def login_body(ticket, attempt_ref, character=None):
    return {
        'protocol_version': 2,
        'game_login_ticket': ticket,
        'attempt_ref': attempt_ref,
        'character_id': character or character_id,
        'channel_id': None,
        'offer': {
            'client_build': 'cutover-smoke',
            'client_platform': 'windows',
            'transports': [{'protocol_major': 1, 'transport_profile': 1, 'alpn': 'oteryn-game/1'}],
        },
    }


# --- 1. An unknown ticket is refused by the Platform issuer --------------------------------------------------
status, _, code = gateway_login(login_body('oteryn-cutover-probe-' + secrets.token_hex(16), uuid7(), uuid7()))
if status != 401 or code != 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED':
    raise SystemExit(f'Native login with an unknown ticket did not reach the Platform issuer: status {status}, code {code}')
if native_login == 'owner-waived':
    # The owner waived the successful native login for this task: no actor and no published route before MariaDB
    # retirement.
    print('Verified the native login refusal of an unknown ticket by the Platform issuer; the successful native login '
          'is owner-waived for this cutover (stated limit: no successful end-to-end native login before MariaDB '
          'retirement).')
    print('native_smoke_unknown_ticket=refused')
    print('native_smoke_login=owner-waived')
    raise SystemExit(0)
if native_login != 'end-to-end':
    raise SystemExit('unknown native login mode')
email = os.environ.get('OTERYN_CUTOVER_NATIVE_SMOKE_EMAIL', '')
password = os.environ.get('OTERYN_CUTOVER_NATIVE_SMOKE_PASSWORD', '')
if not email or not password:
    raise SystemExit('the native smoke actor did not reach the request container')


# --- 2. The smoke actor signs in on the web --------------------------------------------------------------------
class Browser:
    """One web session on the Platform: cookies kept from Set-Cookie, redirects not followed."""

    def __init__(self):
        self.cookies = {}

    def send(self, method, path, form=None):
        headers = {**PROXY_HEADERS, 'Accept': 'text/html'}
        if self.cookies:
            headers['Cookie'] = '; '.join(f'{k}={v}' for k, v in self.cookies.items())
        payload = None
        if form is not None:
            payload = urllib.parse.urlencode(form).encode()
            headers['Content-Type'] = 'application/x-www-form-urlencoded'
            headers['Origin'] = 'https://' + CANONICAL
        status, response_headers, body = exchange(platform_port, method, path, payload, headers, retries=3)
        for value in header(response_headers, 'set-cookie'):
            name, _, rest = value.partition('=')
            self.cookies[name.strip()] = rest.split(';', 1)[0]
        return status, (header(response_headers, 'location') or [''])[0], body

    def token(self, path):
        status, _, body = self.send('GET', path)
        if status != 200:
            raise SystemExit(f'Platform {path.split("?")[0]} returned unexpected status: {status}')
        return field(body, '_token', path)


def field(body, name, where):
    match = (re.search(rb'name="' + name.encode() + rb'"\s+value="([^"]+)"', body)
             or re.search(rb'value="([^"]+)"\s+name="' + name.encode() + rb'"', body))
    if not match:
        raise SystemExit(f'Platform {where.split("?")[0]} has no {name} field')
    return html.unescape(match.group(1).decode())


def canonical_path(location):
    parts = urllib.parse.urlsplit(location)
    if parts.scheme != 'https' or parts.netloc != CANONICAL:
        raise SystemExit('a redirect left the canonical HTTPS origin')
    return parts.path


browser = Browser()
status, location, _ = browser.send('POST', '/login?locale=en', {
    '_token': browser.token('/login?locale=en'), 'email': email, 'password': password})
if status == 302 and canonical_path(location) == '/mfa/challenge':
    raise SystemExit('the native smoke actor has MFA confirmed; the smoke needs an actor without MFA')
if status != 302 or canonical_path(location) != '/':
    raise SystemExit(f'the native smoke actor could not sign in (status {status})')

# --- 3. Native OAuth authorization code with PKCE, as the Windows client does -----------------------------------
verifier = secrets.token_urlsafe(48)
challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).decode().rstrip('=')
state = secrets.token_urlsafe(24)
query = urllib.parse.urlencode({
    'client_id': client_id, 'redirect_uri': REDIRECT_URI, 'response_type': 'code', 'scope': 'game:ticket',
    'state': state, 'code_challenge': challenge, 'code_challenge_method': 'S256'}, quote_via=urllib.parse.quote)
status, location, body = browser.send('GET', '/oauth/authorize?' + query)
if status == 200:
    status, location, _ = browser.send('POST', '/oauth/authorize', {
        '_token': field(body, '_token', '/oauth/authorize'), 'state': state, 'client_id': client_id,
        'auth_token': field(body, 'auth_token', '/oauth/authorize')})
if status != 302 or not location.startswith(REDIRECT_URI + '?'):
    raise SystemExit(f'the native OAuth authorization did not redirect to the client (status {status})')
answer = dict(urllib.parse.parse_qsl(urllib.parse.urlsplit(location).query))
if answer.get('state') != state or not answer.get('code'):
    raise SystemExit(f'the native OAuth authorization returned no code for this state (error {answer.get("error")})')

status, _, body = exchange(platform_port, 'POST', '/oauth/token', urllib.parse.urlencode({
    'grant_type': 'authorization_code', 'client_id': client_id, 'redirect_uri': REDIRECT_URI,
    'code': answer['code'], 'code_verifier': verifier}).encode(), {
    **PROXY_HEADERS, 'Accept': 'application/json', 'Content-Type': 'application/x-www-form-urlencoded'})
token = json_body(body, f'The OAuth token exchange (status {status})')
if status != 200 or not isinstance(token.get('access_token'), str) or str(token.get('token_type', '')).lower() != 'bearer':
    raise SystemExit(f'the OAuth token exchange returned unexpected status: {status}')
bearer = {**PROXY_HEADERS, 'Accept': 'application/json', 'Content-Type': 'application/json',
          'Authorization': 'Bearer ' + token['access_token']}

# --- 4. A native Game Login Ticket; the bearer is single-use ---------------------------------------------------
status, _, body = exchange(platform_port, 'POST', '/api/v1/game-auth/tickets', b'{"protocol_version":1}', bearer)
issued = json_body(body, f'The ticket issue (status {status})')
ticket = issued.get('ticket')
if status != 200 or issued.get('protocol_version') != 1 or not isinstance(ticket, str) or not ticket:
    raise SystemExit(f'the native ticket issue returned unexpected status: {status}')
expires_in = issued.get('expires_in')
if not isinstance(expires_in, int) or isinstance(expires_in, bool) or expires_in < 1:
    raise SystemExit('the native ticket was issued already expired')
status, _, _ = exchange(platform_port, 'POST', '/api/v1/game-auth/tickets', b'{"protocol_version":1}', bearer)
if status != 401:
    raise SystemExit(f'the bearer still issued a ticket after its ticket (status {status}); issuance must revoke it')

# --- 5. Admission through the Gateway, then a replay of the same attempt ----------------------------------------
attempt_ref = uuid7()
request = login_body(ticket, attempt_ref)


def admit(what):
    for attempt in range(6):
        status, answer, code = gateway_login(request)
        if status != 503 or attempt == 5:
            break
        time.sleep(3)
    if status != 200:
        raise SystemExit(f'{what} was not admitted: status {status}, code {code}')
    grant = answer.get('grant')
    endpoint = answer.get('endpoint')
    if (answer.get('protocol_version') != 2 or answer.get('attempt_ref') != attempt_ref
            or not isinstance(answer.get('world_id'), str) or not isinstance(answer.get('channel_id'), str)
            or not isinstance(endpoint, dict) or not endpoint.get('host') or not isinstance(endpoint.get('port'), int)
            or not isinstance(grant, dict) or grant.get('profile') != 'oteryn-pre-admission-v1'
            or not isinstance(grant.get('token'), str) or not grant['token']
            or not isinstance(grant.get('valid_for_seconds'), int) or grant['valid_for_seconds'] < 1):
        raise SystemExit(f'{what} returned 200 without the native admission contract body')
    return answer['world_id'], answer['channel_id'], endpoint


admitted = admit('The native smoke login')
if admit('The replay of the native smoke login') != admitted:
    raise SystemExit('the replay of the admitted attempt returned a different admission')

# --- 6. Sign the web session out -------------------------------------------------------------------------------
status, _, _ = browser.send('POST', '/logout', {'_token': browser.token('/account')})
if status != 302:
    raise SystemExit(f'the native smoke actor could not sign out (status {status})')

print('Verified native login end to end: unknown ticket refused, web sign-in, native OAuth with PKCE, ticket '
      'issue with bearer revocation, Gateway admission with an oteryn-pre-admission-v1 grant, and its replay.')
print('native_smoke_unknown_ticket=refused')
print('native_smoke_oauth=authorized')
print('native_smoke_ticket=issued-bearer-revoked')
print('native_smoke_admission=granted')
print('native_smoke_replay=same-admission')
print('native_smoke_login=end-to-end')
PY

ev_require_continuity "$EV_ATTEMPT_RUN" "$HOOK_WORK/trust.json"

printf 'native_smoke_result=passed\n'
