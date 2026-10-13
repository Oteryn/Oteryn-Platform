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
#   - the admin boundary: the signed-in smoke identity gets 403 from /admin (it has no confirmed MFA and no
#     admin role, so auth then mfa.confirmed refuse it). This proves only the refusal, so:
#   - the administrator, in the running platform service (read-only, before any web write): at least one active
#     identity holds admin.access through its PostgreSQL role rows; its MFA secret decrypts with the deployed
#     application key and is confirmed; the admin route's own middleware (mfa.confirmed, then
#     admin.permission:admin.access) admits it; and the admin dashboard renders for it with its permission
#     navigation. Fails closed when no MFA-confirmed administrator exists: the operator bootstraps the first
#     administrator (admin:bootstrap for an MFA-confirmed identity) before this smoke. It registers no session
#     and writes no security event. Not exercised: the administrator's own password and TOTP entry (that would
#     need the administrator's password and TOTP secret in the workflow); the password sign-in path is the one
#     the smoke identity takes.
#   - the wallet read, independent of the Marketplace switch, in the same in-container probe: the wallet tables
#     through their models (balances and ledger entries load), and the ledger reconciles (every balance equals
#     the sum of its ledger deltas, no ledger entry without its account), in a read-only transaction.
#   - the Marketplace switch matches what is served: when config marketplace.enabled is true, /bazaar is 200
#     and the smoke identity reads /account/bazaar (200); when it is false, /bazaar is 404. Any other pairing
#     fails.
# Output: key=value result lines; never an address, a password, a cookie, a token, an identity id or a body.
set -euo pipefail
umask 077

HOOK_NAME=web-smoke
# shellcheck source=deploy/synology/scripts/cutover-hooks/common.sh
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/common.sh"

(($# == 0)) || hook_fail "takes no arguments."
hook_require_tools docker grep sed
hook_workdir
hook_runtime_env
port="$(hook_platform_port)"

hook_platform_php > "$HOOK_WORK/probe.out" <<'PHP' || hook_fail "the administrator and wallet probe failed."
<?php
require '/var/www/html/vendor/autoload.php';
$app = require '/var/www/html/bootstrap/app.php';
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();

use App\Admin\AdminAuthorization;
use App\Admin\AdminPermission;
use App\Http\Middleware\EnsureConfirmedMfa;
use App\Http\Middleware\RequireAdminPermission;
use App\Identity\Models\Identity;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Illuminate\Contracts\Encryption\DecryptException;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\ViewErrorBag;
use Symfony\Component\HttpFoundation\Response as SymfonyResponse;
use Symfony\Component\HttpKernel\Exception\HttpExceptionInterface;

function smoke_fail(string $message, int $code): never
{
    fwrite(STDERR, $message."\n");
    exit($code);
}

function smoke_admin(Illuminate\Foundation\Application $app): void
{
    // --- Administrator: role loading, MFA confirmation and an authorized admin page read -------------------
    $holders = DB::table('identity_admin_roles')
        ->join('admin_role_permissions', 'admin_role_permissions.role_id', '=', 'identity_admin_roles.role_id')
        ->join('admin_permissions', 'admin_permissions.id', '=', 'admin_role_permissions.permission_id')
        ->where('admin_permissions.key', AdminPermission::ACCESS)
        ->distinct()
        ->orderBy('identity_admin_roles.identity_id')
        ->pluck('identity_admin_roles.identity_id')
        ->all();
    if ($holders === []) {
        smoke_fail('No identity holds admin.access: bootstrap the first administrator (admin:bootstrap for an MFA-confirmed identity) before the smoke.', 41);
    }
    $authorization = $app->make(AdminAuthorization::class);
    $admin = null;
    foreach ($holders as $holder) {
        $identity = Identity::query()->find($holder);
        if (! $identity instanceof Identity || $identity->disabled_at !== null || $identity->isTerminated()) {
            continue;
        }
        try {
            $confirmed = $identity->hasConfirmedMfa();
        } catch (DecryptException) {
            smoke_fail('An administrator MFA secret does not decrypt with the deployed application key.', 42);
        }
        if ($confirmed && $authorization->allows($identity, AdminPermission::ACCESS)) {
            $admin = $identity;
            break;
        }
    }
    if (! $admin instanceof Identity) {
        smoke_fail('No active administrator with confirmed MFA holds admin.access: confirm MFA for an administrator before the smoke.', 43);
    }
    if (! in_array(AdminPermission::ACCESS, $authorization->grantedPermissions($admin), true)) {
        smoke_fail('The administrator permission projection does not list admin.access.', 44);
    }

    // The admin route's own middleware (mfa.confirmed, then admin.permission:admin.access) on a request whose
    // user is that administrator, then the dashboard view with its permission navigation. Read-only: no session is
    // registered and no security event is written, so the administrator's sign-in state is untouched.
    $request = Request::create('https://oteryn.molehill.cloud/admin', 'GET');
    $request->setUserResolver(static fn (): Identity => $admin);
    $app->instance('request', $request);
    $auth = $app->make('auth');
    $auth->shouldUse('web');
    $auth->guard('web')->setUser($admin);
    try {
        $passed = $app->make(EnsureConfirmedMfa::class)->handle(
            $request,
            static fn (Request $inner): SymfonyResponse => $app->make(RequireAdminPermission::class)->handle(
                $inner,
                static fn (): SymfonyResponse => new SymfonyResponse('admitted'),
                AdminPermission::ACCESS,
            ),
        );
    } catch (HttpExceptionInterface $refused) {
        smoke_fail("The admin route middleware refused the administrator (status {$refused->getStatusCode()}).", 45);
    }
    if ($passed->getStatusCode() !== 200 || $passed->getContent() !== 'admitted') {
        smoke_fail('The admin route middleware did not admit the administrator.', 45);
    }
    $html = view('admin.dashboard', ['errors' => new ViewErrorBag])->render();
    if (! str_contains($html, 'admin-task-grid') || ! str_contains($html, 'https://oteryn.molehill.cloud/admin')) {
        smoke_fail('The admin dashboard did not render for the administrator.', 46);
    }
    echo "web_smoke_admin_access=granted\n";
    echo "web_smoke_admin_mfa=confirmed\n";
    echo "web_smoke_admin_page=read\n";
}

function smoke_wallet(): void
{
    // --- Wallet read, independent of the Marketplace switch ----------------------------------------------
    // Read-only: the wallet tables through their models, and the ledger reconciliation (every balance is the sum
    // of its ledger deltas, every ledger entry has its account), in one transaction that is rolled back.
    DB::beginTransaction();
    try {
        if (DB::getDriverName() === 'pgsql') {
            DB::statement('SET TRANSACTION READ ONLY');
        }
        $accounts = WalletAccount::query()->count();
        $entries = WalletLedgerEntry::query()->count();
        foreach (WalletAccount::query()->orderBy('identity_id')->limit(50)->get() as $wallet) {
            if (! is_int($wallet->available_balance) || ! is_int($wallet->reserved_balance)
                || $wallet->available_balance < 0 || $wallet->reserved_balance < 0) {
                smoke_fail('A wallet account holds an invalid balance.', 51);
            }
        }
        foreach (WalletLedgerEntry::query()->orderBy('id')->limit(50)->get() as $entry) {
            if (! is_int($entry->available_delta) || ! is_int($entry->reserved_delta) || ! is_string($entry->idempotency_key)) {
                smoke_fail('A wallet ledger entry did not load.', 52);
            }
        }
        $sums = DB::table('wallet_ledger_entries')
            ->select('identity_id')
            ->selectRaw('SUM(available_delta) AS available_sum, SUM(reserved_delta) AS reserved_sum')
            ->groupBy('identity_id');
        $unreconciled = DB::table('wallet_accounts')
            ->leftJoinSub($sums, 'ledger', 'ledger.identity_id', '=', 'wallet_accounts.identity_id')
            ->whereRaw('wallet_accounts.available_balance <> COALESCE(ledger.available_sum, 0)')
            ->orWhereRaw('wallet_accounts.reserved_balance <> COALESCE(ledger.reserved_sum, 0)')
            ->count();
        $orphans = DB::table('wallet_ledger_entries')
            ->leftJoin('wallet_accounts', 'wallet_accounts.identity_id', '=', 'wallet_ledger_entries.identity_id')
            ->whereNull('wallet_accounts.identity_id')
            ->count();
    } finally {
        DB::rollBack();
    }
    if ($unreconciled !== 0 || $orphans !== 0) {
        smoke_fail("The wallet ledger does not reconcile ({$unreconciled} accounts differ from their ledger, {$orphans} entries without an account).", 53);
    }
    echo "web_smoke_wallet_accounts={$accounts}\n";
    echo "web_smoke_wallet_ledger_entries={$entries}\n";
    echo "web_smoke_wallet=read-reconciled\n";
    echo 'web_smoke_marketplace_enabled='.(config('marketplace.enabled') === true ? 'true' : 'false')."\n";
}

// An exception the probe does not expect fails it by class only (the console handler would exit 0, and a
// message could carry data).
try {
    smoke_admin($app);
    smoke_wallet();
} catch (Throwable $unexpected) {
    smoke_fail('The admin and wallet probe failed: '.$unexpected::class, 40);
}
PHP
probe="$(cat -- "$HOOK_WORK/probe.out")"
# The console exception handler can exit 0, so the probe passes only with every result line.
for line in web_smoke_admin_access=granted web_smoke_admin_mfa=confirmed web_smoke_admin_page=read \
    web_smoke_wallet=read-reconciled; do
    grep -qx "$line" <<<"$probe" || hook_fail "the administrator and wallet probe did not report $line."
done
for pattern in 'web_smoke_wallet_accounts=[0-9]+' 'web_smoke_wallet_ledger_entries=[0-9]+'; do
    grep -qxE "$pattern" <<<"$probe" || hook_fail "the wallet probe did not report its counts."
done
marketplace="$(sed -nE 's/^web_smoke_marketplace_enabled=(true|false)$/\1/p' <<<"$probe" | tail -n 1)"
[[ "$marketplace" == true || "$marketplace" == false ]] || hook_fail "the probe did not report the Marketplace switch."
grep -E '^web_smoke_(admin_(access|mfa|page)|wallet(_accounts|_ledger_entries)?)=[a-z0-9-]+$' <<<"$probe"

hook_loopback_python "$port" "$marketplace" <<'PY' || hook_fail "the Platform web smoke failed."
import http.client
import re
import secrets
import sys
import time
import urllib.parse

port = int(sys.argv[1])
marketplace_enabled = sys.argv[2] == 'true'
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
if marketplace_enabled:
    if status != 200:
        raise SystemExit(f'/bazaar returned {status} while the Marketplace is enabled')
    status, _, _ = browser.send('GET', '/account/bazaar')
    if status != 200:
        raise SystemExit(f'the Bazaar account page (/account/bazaar) returned unexpected status: {status}')
    marketplace = 'read'
elif status == 404:
    marketplace = 'off'
else:
    raise SystemExit(f'/bazaar returned {status} while the Marketplace is disabled, not 404')

status, location, _ = browser.send('POST', '/logout', {'_token': logout_token})
if status != 302:
    raise SystemExit(f'sign-out returned unexpected status: {status}')
status, location, _ = browser.send('GET', '/account')
if status != 302 or redirect_path(location) != '/login':
    raise SystemExit('the session still reads /account after sign-out')

print('Verified Platform /health, login form, public news, registration, sign-in, account read, '
      'the admin boundary and sign-out over host loopback.')
print('web_smoke_account=registered-signed-in-signed-out')
print('web_smoke_admin_boundary=refused-without-mfa')
print(f'web_smoke_marketplace={marketplace}')
PY

printf 'web_smoke_result=passed\n'
