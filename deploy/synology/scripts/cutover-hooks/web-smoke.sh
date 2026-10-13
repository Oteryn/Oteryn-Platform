#!/usr/bin/env bash
# Cutover hook OTERYN_CUTOVER_WEB_SMOKE_COMMAND (plan P6 step 3). cutover-postgres.sh smoke runs it, with no
# arguments, right after health-check.sh passes on the PostgreSQL-backed stack. It adds a short loopback
# web smoke of the public Platform paths that read the database, through the same host-loopback origin and
# canonical proxy headers as health-check.sh:
#   - GET /health returns 200;
#   - GET /login?locale=en returns 200, keeps the canonical HTTPS form action, carries a CSRF field and
#     starts a session (Set-Cookie);
#   - GET /news?locale=en returns 200 HTML (public CMS read).
# Read-only: it submits no form and changes no state. Output: one result line; never a cookie or body.
set -euo pipefail
umask 077

HOOK_NAME=web-smoke
# shellcheck source=deploy/synology/scripts/cutover-hooks/common.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

(($# == 0)) || hook_fail "takes no arguments."
hook_require_tools docker sed
hook_workdir
hook_runtime_env
port="$(hook_platform_port)"

hook_loopback_python "$port" <<'PY' || hook_fail "the Platform web smoke failed."
import http.client
import sys
import time

port = int(sys.argv[1])
CANONICAL = 'oteryn.molehill.cloud'
PROXY_HEADERS = {
    'Host': CANONICAL,
    'X-Forwarded-Host': CANONICAL,
    'X-Forwarded-Proto': 'https',
    'X-Forwarded-Port': '443',
}


def request(path, accept):
    last_error = None
    for attempt in range(15):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=5)
        try:
            connection.request('GET', path, headers={**PROXY_HEADERS, 'Accept': accept})
            response = connection.getresponse()
            body = response.read(262144)
            return response.status, response.getheaders(), body
        except OSError as exc:
            last_error = exc
            if attempt == 14:
                break
            time.sleep(2)
        finally:
            connection.close()
    raise SystemExit(f'Host-loopback request failed after bounded retries: GET {path} ({type(last_error).__name__})')


def header(headers, name):
    return [value for key, value in headers if key.lower() == name]


status, _, _ = request('/health', 'application/json')
if status != 200:
    raise SystemExit(f'Platform /health returned unexpected status: {status}')

status, headers, body = request('/login?locale=en', 'text/html')
if status != 200:
    raise SystemExit(f'Platform /login returned unexpected status: {status}')
if b'action="https://oteryn.molehill.cloud/login?locale=en"' not in body:
    raise SystemExit('Platform login form did not keep the canonical HTTPS form action')
if b'name="_token"' not in body:
    raise SystemExit('Platform login form has no CSRF field')
if not header(headers, 'set-cookie'):
    raise SystemExit('Platform login page started no session')

status, headers, body = request('/news?locale=en', 'text/html')
if status != 200:
    raise SystemExit(f'Platform /news returned unexpected status: {status}')
if not any(value.lower().startswith('text/html') for value in header(headers, 'content-type')):
    raise SystemExit('Platform /news did not return HTML')

print('Verified Platform /health, login form (canonical action, CSRF, session) and public news over host loopback.')
PY

printf 'web_smoke_result=passed\n'
