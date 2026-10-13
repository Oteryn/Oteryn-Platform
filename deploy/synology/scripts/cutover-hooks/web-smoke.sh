#!/usr/bin/env bash
# Cutover hook OTERYN_CUTOVER_WEB_SMOKE_COMMAND (plan P6 step 3: health-check.sh, web registration and login,
# admin login and a wallet read). cutover-postgres.sh smoke runs it, with no arguments, right after
# health-check.sh passes on the PostgreSQL-backed stack, before the last rollback point. Through the same
# host-loopback origin and canonical proxy headers as health-check.sh it checks:
#   - GET /health returns 200;
#   - GET /login?locale=en returns 200, keeps the canonical HTTPS form action, carries a CSRF field and
#     starts a session (Set-Cookie);
#   - GET /news?locale=en returns 200 HTML (public CMS read);
#   - registration and login, actor to result: a new smoke identity (random example.invalid address and
#     random password, both never printed or kept) is registered through the CSRF-protected form, signs in
#     through /login (redirect to /), reads /account, which shows its address, and signs out (after which
#     /account redirects to /login). These are writes to the PostgreSQL identity, session and security-event
#     tables; none touches the native-evidence witness. The smoke identity stays in the database (test data).
#   - the admin boundary: the signed-in smoke identity gets 403 from /admin (auth, then mfa.confirmed).
#   - the wallet read: the only wallet read path on the web is the Character Bazaar account page. When the
#     Bazaar is enabled, the smoke identity must read /account/bazaar (200); when it is disabled, /bazaar must
#     be 404, so no wallet surface is served at all.
# Declared limitation: admin login itself is not exercised. A fresh P3 bootstrap has no administrator;
# admin:bootstrap assigns the one-time first platform_admin to an MFA-confirmed identity, which the operator
# does after the cutover, and a smoke must neither consume that assignment nor hold an admin password or TOTP
# secret. The hook proves the admin boundary instead and prints web_smoke_admin_login=not-exercised.
# Output: key=value result lines; never an address, a password, a cookie, a token or a body.
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
import re
import secrets
import sys
import time
import urllib.parse

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


# --- registration and login, actor to result -------------------------------------------------------------
class Browser:
    """One web session: cookies kept from Set-Cookie, redirects not followed."""

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
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=10)
        try:
            connection.request(method, path, body=payload, headers=headers)
            response = connection.getresponse()
            body = response.read(1048576)
            status, response_headers = response.status, response.getheaders()
        except OSError as exc:
            raise SystemExit(f'Host-loopback request failed: {method} {path.split("?")[0]} ({type(exc).__name__})')
        finally:
            connection.close()
        for value in header(response_headers, 'set-cookie'):
            name, _, rest = value.partition('=')
            self.cookies[name.strip()] = rest.split(';', 1)[0]
        location = (header(response_headers, 'location') or [''])[0]
        return status, location, body

    def token(self, path):
        status, _, body = self.send('GET', path)
        if status != 200:
            raise SystemExit(f'Platform {path.split("?")[0]} returned unexpected status: {status}')
        match = re.search(rb'name="_token"\s+value="([^"]+)"', body) or re.search(rb'value="([^"]+)"\s+name="_token"', body)
        if not match:
            raise SystemExit(f'Platform {path.split("?")[0]} has no CSRF field')
        return match.group(1).decode()


def redirect_path(location):
    parts = urllib.parse.urlsplit(location)
    if parts.scheme != 'https' or parts.netloc != CANONICAL:
        raise SystemExit('a redirect left the canonical HTTPS origin')
    return parts.path


email = f'cutover-smoke-{secrets.token_hex(8)}@example.invalid'
password = secrets.token_urlsafe(24) + 'Aa1!'
browser = Browser()

status, location, _ = browser.send('POST', '/register', {
    '_token': browser.token('/register?locale=en'), 'email': email,
    'password': password, 'password_confirmation': password})
if status == 429:
    raise SystemExit('registration was throttled; wait a minute and run the smoke again')
if status != 302 or redirect_path(location) != '/register':
    raise SystemExit(f'registration returned unexpected status: {status}')

status, location, _ = browser.send('POST', '/login?locale=en', {
    '_token': browser.token('/login?locale=en'), 'email': email, 'password': password})
if status != 302 or redirect_path(location) != '/':
    raise SystemExit(f'the registered smoke identity could not sign in (status {status})')

status, _, body = browser.send('GET', '/account')
if status != 200 or email.encode() not in body:
    raise SystemExit(f'the signed-in smoke identity could not read its account (status {status})')
match = re.search(rb'name="_token"\s+value="([^"]+)"', body) or re.search(rb'value="([^"]+)"\s+name="_token"', body)
if not match:
    raise SystemExit('the account page has no sign-out form')
logout_token = match.group(1).decode()

status, _, _ = browser.send('GET', '/admin')
if status != 403:
    raise SystemExit(f'/admin returned {status} to a signed-in identity without MFA or an admin role, not 403')

status, _, _ = browser.send('GET', '/bazaar')
if status == 404:
    wallet = 'no-route'
elif status == 200:
    status, _, _ = browser.send('GET', '/account/bazaar')
    if status != 200:
        raise SystemExit(f'the wallet read (/account/bazaar) returned unexpected status: {status}')
    wallet = 'read'
else:
    raise SystemExit(f'/bazaar returned unexpected status: {status}')

status, location, _ = browser.send('POST', '/logout', {'_token': logout_token})
if status != 302:
    raise SystemExit(f'sign-out returned unexpected status: {status}')
status, location, _ = browser.send('GET', '/account')
if status != 302 or redirect_path(location) != '/login':
    raise SystemExit('the session still reads /account after sign-out')

print('Verified Platform /health, login form, public news, registration, sign-in, account read, '
      'the admin boundary and sign-out over host loopback.')
print('web_smoke_account=registered-signed-in-signed-out')
print('web_smoke_admin_boundary=refused')
print('web_smoke_admin_login=not-exercised')
print(f'web_smoke_wallet={wallet}')
PY

printf 'web_smoke_result=passed\n'
