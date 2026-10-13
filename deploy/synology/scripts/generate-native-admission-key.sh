#!/usr/bin/env bash
# Generates and rotates the Platform native admission signing key (Ed25519) on the Synology host.
#
# Custody: the key lives only in the Docker named volume oteryn-native-admission-keys, which Docker
# stores under its root-only data root, outside the repository, the Compose context, the rendered
# .env and the runner state directory. Inside the volume the directory is 0700 and each key file is
# 0600, owned by the Platform process user (the keyring refuses any other owner or mode). The
# private key (seed) is generated and read inside a throwaway container with no network; it is never
# printed, logged, passed on a command line or written anywhere else. Only the key id and the PUBLIC
# key (base64url, the encoding of the Platform trust registry and Game's trust config) are printed.
#
# Usage: generate-native-admission-key.sh [ensure|stage|promote|rotate|drop-retiring|show|check|check-kid-history]
#   stage          generate the NEXT key (never loaded by Platform, so nothing changes at runtime);
#                  idempotent; prints its key id and public key for Game's trust configuration
#   promote        staged rotation: with ingress stopped, next -> current and current -> retiring,
#                  recreate Platform, publish and prove trust, only then start ingress again
#                  (needs OTERYN_ENV_FILE; no signing with an untrusted key, no maintenance flag)
#   ensure         create the current key if none exists; otherwise change nothing (idempotent)
#   rotate         current -> retiring, then a new current with a never-used key id
#   drop-retiring  remove the retiring key once every attempt it signed has passed exp
#   show           print the key ids and public keys, changing nothing
#   check-kid-history [--kid <kid>...]
#                  read-only (P6a D7): print how many used.kids records exist for each role key
#                  (current, next, retiring) and each --kid; exit non-zero unless every count is 1
#   check          run the Platform signing self-check in the running Platform container
#
# rotate moves the trusted current key to retiring and restarts Platform with a new current key whose
# public key is not trusted yet, so native login fails until it is published. While native admission
# is live (a running Platform container has GAME_AUTH_NATIVE_ADMISSION_ENABLED=true) rotate therefore
# refuses unless called as "rotate --maintenance": the operator accepts a native login outage from the
# rotate until the new public key is published and check passes (set OTERYN_STAGING_NATIVE_ADMISSION_ENABLED
# to false and redeploy first to avoid the outage).
#
# rotate and drop-retiring restart the running Platform container(s): the entrypoint exports the key
# ids once at start, so a running container would otherwise keep ids that now name other files.
# After rotate, signing needs the new public key in the Platform trust registry (a redeploy publishes
# it); run check to confirm.
#
# The Platform image comes from PLATFORM_IMAGE (or OTERYN_ENV_FILE). Rotation order follows the
# native login contract 9.3: rotate, publish the new public key to the Platform trust registry and
# Game's trust config, wait for trust to propagate, and only then drop-retiring after the retiring
# key's grants have expired.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
VOLUME_NAME="oteryn-native-admission-keys"
ACTION="${1:-ensure}"
MAINTENANCE="${2:-}"

case "$ACTION" in
    ensure | stage | promote | rotate | drop-retiring | show | check | check-kid-history) ;;
    *)
        echo "Usage: $0 [ensure|stage|promote|rotate [--maintenance]|drop-retiring|show|check|check-kid-history [--kid <kid>...]]" >&2
        exit 2
        ;;
esac

if [[ "$ACTION" == promote ]]; then
    # Validated before any mutation: the wait is the only thing that keeps ingress closed until Game's
    # trust cache has seen the new key (contract 9.3: five seconds plus the clock uncertainty bound).
    propagation_seconds="${OTERYN_NATIVE_TRUST_PROPAGATION_SECONDS:-10}"
    if [[ ! "$propagation_seconds" =~ ^[0-9]{1,5}$ ]] || ((10#$propagation_seconds < 10)); then
        echo "OTERYN_NATIVE_TRUST_PROPAGATION_SECONDS must be an integer of at least 10 (contract 9.3: five seconds plus clock uncertainty)." >&2
        exit 2
    fi
    propagation_seconds=$((10#$propagation_seconds))
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required to manage the native admission key volume." >&2
    exit 1
fi

if [[ -z "${PLATFORM_IMAGE:-}" && -n "${OTERYN_ENV_FILE:-}" ]]; then
    # shellcheck source=deploy/synology/scripts/lib.sh
    source "$SCRIPT_DIR/lib.sh"
    load_oteryn_env_file "$OTERYN_ENV_FILE"
fi
if [[ -z "${PLATFORM_IMAGE:-}" || "$PLATFORM_IMAGE" == REQUIRED_* ]]; then
    echo "PLATFORM_IMAGE (or OTERYN_ENV_FILE) must name the Platform image that provides PHP and libsodium." >&2
    exit 1
fi

platform_containers() {
    docker ps -q \
        --filter "label=com.docker.compose.project=${COMPOSE_PROJECT_NAME:-oteryn-staging}" \
        --filter "label=com.docker.compose.service=platform"
}

if [[ "$ACTION" == check ]]; then
    # shellcheck source=deploy/synology/scripts/lib.sh
    source "$SCRIPT_DIR/lib.sh"
    container="$(platform_containers | head -n 1)"
    oteryn_run_native_admission_self_check "$container"
    exit $?
fi

if [[ "$ACTION" == check-kid-history ]] && ! docker volume inspect "$VOLUME_NAME" >/dev/null 2>&1; then
    echo "The native admission key volume '$VOLUME_NAME' does not exist; the read-only history check will not create it." >&2
    exit 1
fi

docker volume create "$VOLUME_NAME" >/dev/null

# Runs inside the Platform image as root only to set ownership; no network, no extra privileges.
read -r -d '' PHP_TOOL <<'PHP' || true
error_reporting(E_ALL);
ini_set('display_errors', 'stderr');
umask(0077);

$dir = '/keys';
$action = $argv[1] ?? '';
$user = posix_getpwnam('www-data');
if ($user === false) {
    fwrite(STDERR, "The Platform process user does not exist in the image.\n");
    exit(1);
}
$uid = (int) $user['uid'];
$gid = (int) $user['gid'];

function b64u(string $bytes): string
{
    return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
}

function fail(string $message): never
{
    fwrite(STDERR, $message . "\n");
    exit(1);
}

function secure(string $path, int $uid, int $gid, int $mode): void
{
    if (! chown($path, $uid) || ! chgrp($path, $gid) || ! chmod($path, $mode)) {
        fail('Unable to set ownership on a key custody path.');
    }
}

function writeSecret(string $dir, string $name, string $contents, int $uid, int $gid): void
{
    $tmp = $dir . '/.' . $name . '.tmp';
    @unlink($tmp);
    $handle = fopen($tmp, 'xb');
    if ($handle === false) {
        fail('Unable to create a key custody file.');
    }
    $written = fwrite($handle, $contents);
    fclose($handle);
    if ($written !== strlen($contents)) {
        @unlink($tmp);
        fail('Unable to write a key custody file.');
    }
    secure($tmp, $uid, $gid, 0600);
    if (! rename($tmp, $dir . '/' . $name)) {
        @unlink($tmp);
        fail('Unable to publish a key custody file.');
    }
}

/** @return array{string, string}|null key id and public key of a role, null when absent */
function load(string $dir, string $role, int $uid): ?array
{
    $keyFile = $dir . '/' . $role . '.key';
    $kidFile = $dir . '/' . $role . '.kid';
    // A symlink counts as present (even a dangling one) so it is refused below, never skipped.
    $haveKey = is_file($keyFile) || is_link($keyFile);
    $haveKid = is_file($kidFile) || is_link($kidFile);
    if (! $haveKey && ! $haveKid) {
        return null;
    }
    if (! $haveKey || ! $haveKid || is_link($keyFile) || is_link($kidFile) || ! is_file($keyFile) || ! is_file($kidFile)) {
        fail("The $role key custody is incomplete; refusing to continue.");
    }
    foreach ([$keyFile, $kidFile] as $file) {
        $stat = stat($file);
        if ($stat === false || ($stat['mode'] & 0o077) !== 0 || $stat['uid'] !== $uid) {
            fail("The $role key custody has unsafe ownership or permissions; refusing to continue.");
        }
    }
    $kid = trim((string) file_get_contents($kidFile));
    $seed = trim((string) file_get_contents($keyFile));
    if (preg_match('/\A[A-Za-z0-9._-]{1,64}\z/', $kid) !== 1 || preg_match('/\A[A-Za-z0-9_-]{43}\z/', $seed) !== 1) {
        fail("The $role key custody holds malformed content; refusing to continue.");
    }
    $raw = base64_decode(strtr($seed . '=', '-_', '+/'), true);
    if (! is_string($raw) || strlen($raw) !== SODIUM_CRYPTO_SIGN_SEEDBYTES || b64u($raw) !== $seed) {
        fail("The $role key is not a canonical Ed25519 seed; refusing to continue.");
    }
    $public = sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($raw));
    sodium_memzero($raw);

    return [$kid, b64u($public)];
}

function usedKids(string $dir, int $uid): array
{
    $file = $dir . '/used.kids';
    if (! is_file($file) && ! is_link($file)) {
        return [];
    }
    // Same custody rules as load(): a regular file (never a symlink) owned by the process user, 0600.
    $stat = is_link($file) || ! is_file($file) ? false : stat($file);
    if ($stat === false || ($stat['mode'] & 0o077) !== 0 || $stat['uid'] !== $uid) {
        fail('The key id history has an unsafe type, ownership or permissions; refusing to continue.');
    }

    return array_filter(array_map('trim', file($file, FILE_IGNORE_NEW_LINES) ?: []));
}

function generate(string $dir, string $role, int $uid, int $gid): void
{
    $used = usedKids($dir, $uid);
    do {
        $kid = 'oteryn-admission-' . gmdate('Ymd-His') . '-' . bin2hex(random_bytes(3));
    } while (in_array($kid, $used, true));
    $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
    writeSecret($dir, $role . '.key', b64u($seed) . "\n", $uid, $gid);
    sodium_memzero($seed);
    writeSecret($dir, $role . '.kid', $kid . "\n", $uid, $gid);
    $handle = fopen($dir . '/used.kids', 'ab');
    if ($handle === false || fwrite($handle, $kid . "\n") === false) {
        fail('Unable to record the key id history.');
    }
    fclose($handle);
    secure($dir . '/used.kids', $uid, $gid, 0600);
}

function report(string $dir, int $uid): void
{
    $current = load($dir, 'current', $uid);
    if ($current === null) {
        fail('No native admission signing key exists.');
    }
    echo 'current_key_id=' . $current[0] . "\n";
    echo 'current_public_key=' . $current[1] . "\n";
    $next = load($dir, 'next', $uid);
    if ($next !== null) {
        echo 'next_key_id=' . $next[0] . "\n";
        echo 'next_public_key=' . $next[1] . "\n";
    }
    $retiring = load($dir, 'retiring', $uid);
    if ($retiring !== null) {
        echo 'retiring_key_id=' . $retiring[0] . "\n";
        echo 'retiring_public_key=' . $retiring[1] . "\n";
    }
}

if (! is_dir($dir)) {
    fail('The key volume is not mounted.');
}
if ($action === 'check-kid-history') {
    // Read-only (P6a D7): changes nothing on the volume. Each role key and each --kid must have
    // exactly one used.kids record. Role key ids are read only through load() (regular non-symlink
    // files, owner and mode, canonical seed, key id format) and every key id is validated before
    // anything is printed, so custody content that fails validation never reaches the output.
    $counts = array_count_values(usedKids($dir, $uid));
    $kids = [];
    foreach (['current', 'next', 'retiring'] as $role) {
        $loaded = load($dir, $role, $uid);
        if ($loaded !== null) {
            $kids[$role] = $loaded[0];
        }
    }
    $extra = array_slice($argv, 2);
    for ($i = 0; $i < count($extra); $i += 2) {
        if (($extra[$i] ?? '') !== '--kid' || ! isset($extra[$i + 1]) || preg_match('/\A[A-Za-z0-9._-]{1,64}\z/', $extra[$i + 1]) !== 1) {
            fail('check-kid-history accepts only --kid <kid> arguments with a well-formed key id.');
        }
        $kids['kid:' . $extra[$i + 1]] = $extra[$i + 1];
    }
    $ok = $kids !== [];
    foreach ($kids as $label => $kid) {
        $count = $counts[$kid] ?? 0;
        echo 'kid_history ' . $label . ' ' . $kid . ' count=' . $count . "\n";
        $ok = $ok && $count === 1;
    }
    exit($ok ? 0 : 1);
}
secure($dir, $uid, $gid, 0700);

switch ($action) {
    case 'ensure':
        if (load($dir, 'current', $uid) === null) {
            generate($dir, 'current', $uid, $gid);
            fwrite(STDERR, "Generated the native admission signing key.\n");
        }
        break;
    case 'stage':
        if (load($dir, 'current', $uid) === null) {
            fail('There is no current key; run ensure first.');
        }
        if (load($dir, 'next', $uid) === null) {
            generate($dir, 'next', $uid, $gid);
            fwrite(STDERR, "Staged the next native admission signing key.\n");
        }
        break;
    case 'promote':
        if (load($dir, 'next', $uid) === null) {
            fail('No staged next key; run stage first.');
        }
        if (load($dir, 'retiring', $uid) !== null) {
            fail('A retiring key is still present; run drop-retiring once its grants have expired.');
        }
        // Transactional: every rename is undone in reverse order when one fails. Exit 4 tells the
        // caller custody may have changed (it must keep ingress closed); exit 1 means nothing changed.
        $moves = [];
        foreach (['key', 'kid'] as $suffix) {
            $moves[] = ["$dir/current.$suffix", "$dir/retiring.$suffix"];
        }
        foreach (['key', 'kid'] as $suffix) {
            $moves[] = ["$dir/next.$suffix", "$dir/current.$suffix"];
        }
        $done = [];
        foreach ($moves as [$from, $to]) {
            if (! rename($from, $to)) {
                $clean = true;
                foreach (array_reverse($done) as [$undoFrom, $undoTo]) {
                    $clean = rename($undoTo, $undoFrom) && $clean;
                }
                fwrite(STDERR, $clean ? "Unable to promote the staged key; custody restored.\n" : "Unable to promote the staged key and custody could not be restored.\n");
                exit($clean && $done === [] ? 1 : 4);
            }
            $done[] = [$from, $to];
        }
        fwrite(STDERR, "Promoted: the staged key is now current and the previous key is retiring.\n");
        break;
    case 'rotate':
        if (load($dir, 'current', $uid) === null) {
            fail('There is no current key to rotate; run ensure first.');
        }
        if (load($dir, 'retiring', $uid) !== null) {
            fail('A retiring key is still present; run drop-retiring once its grants have expired.');
        }
        foreach (['key', 'kid'] as $suffix) {
            if (! rename("$dir/current.$suffix", "$dir/retiring.$suffix")) {
                fail('Unable to retire the current key.');
            }
        }
        generate($dir, 'current', $uid, $gid);
        fwrite(STDERR, "Rotated: the previous key is now retiring.\n");
        break;
    case 'drop-retiring':
        foreach (['key', 'kid'] as $suffix) {
            @unlink("$dir/retiring.$suffix");
        }
        break;
    case 'show':
        break;
    default:
        fail('Unknown action.');
}

report($dir, $uid);
PHP

if [[ "$ACTION" == rotate && "$MAINTENANCE" != --maintenance ]]; then
    while IFS= read -r container; do
        if [[ -n "$container" ]] && docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$container" \
            | grep -qx 'GAME_AUTH_NATIVE_ADMISSION_ENABLED=true'; then
            echo "Refusing to rotate while native admission is live: the new key is untrusted until its public key is published, so native login would fail. Disable native admission and redeploy, or run '$0 rotate --maintenance' to accept the outage." >&2
            exit 3
        fi
    done < <(platform_containers)
fi

run_key_tool() {
    docker run --rm \
        --user 0:0 \
        --network none \
        --read-only \
        --cap-drop ALL --cap-add CHOWN --cap-add FOWNER --cap-add DAC_OVERRIDE \
        --security-opt no-new-privileges \
        --entrypoint php \
        --volume "$VOLUME_NAME:/keys" \
        "$PLATFORM_IMAGE" -d display_errors=stderr -r "$PHP_TOOL" -- "$@"
}

if [[ "$ACTION" == promote ]]; then
    # Staged rotation. Before promoting, add the key printed by stage to Game's trust configuration and
    # wait for it to propagate (contract 9.3). Promotion never lets Platform sign with an untrusted
    # key: ingress stays stopped until the recreated Platform has published the new key to the trust
    # registry, passed the self-check and the Game trust cache interval (default 10 s, five seconds
    # plus the clock uncertainty bound) has elapsed.
    [[ -n "${OTERYN_ENV_FILE:-}" ]] || { echo "promote needs OTERYN_ENV_FILE (the deployment environment file)." >&2; exit 2; }
    # shellcheck source=deploy/synology/scripts/lib.sh
    source "$SCRIPT_DIR/lib.sh"
    # The deployment environment decides the flag, whatever PLATFORM_IMAGE was preselected.
    load_oteryn_env_file "$OTERYN_ENV_FILE"
    DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
    ENV_FILE="$OTERYN_ENV_FILE"
    COMPOSE_FILE="$DEPLOY_DIR/compose.yml"
    case "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}" in
        true | false) ;;
        *) echo "OTERYN_NATIVE_ADMISSION_ENABLED must be true or false." >&2; exit 2 ;;
    esac
    promote_compose=(command docker compose --env-file "$OTERYN_ENV_FILE" -f "$COMPOSE_FILE")
    ingress="$(oteryn_native_admission_ingress_stop "${promote_compose[@]}")"
    promote_rc=0
    run_key_tool promote || promote_rc=$?
    if ((promote_rc == 1)); then
        # Nothing changed: the preserved ingress starts again.
        # shellcheck disable=SC2086 # container ids, one per line
        oteryn_native_admission_ingress_start $ingress
        exit 1
    fi
    # From here on custody may have changed. Any failure removes Platform, the internal proxy and
    # Gateway (they are `restart: always`), so nothing can start behind an unproven key.
    promote_fail_closed() {
        echo "Promotion failed; removing Platform, internal proxy and Gateway (fail closed)." >&2
        "${promote_compose[@]}" rm -s -f gateway internal-proxy platform >&2 || true
    }
    trap 'rc=$?; ((rc == 0)) || promote_fail_closed; exit "$rc"' EXIT
    ((promote_rc == 0)) || exit "$promote_rc"
    "${promote_compose[@]}" up -d --no-deps --force-recreate platform
    oteryn_publish_native_admission_trust_or_stop "${promote_compose[@]}" || exit 1
    # Marketplace reconciliation may force-recreate Platform again; it self-checks that final
    # replacement itself (lib.sh) before ingress can return, so the reconcile runs first and the
    # self-check below proves the Platform that will really serve.
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change || exit 1
    oteryn_native_admission_self_check_or_stop "${promote_compose[@]}" || exit 1
    sleep "$propagation_seconds"
    # shellcheck disable=SC2086
    oteryn_native_admission_ingress_start $ingress || exit 1
    trap - EXIT
    echo "Promoted the staged key; Platform restarted behind the trust gate and ingress is back." >&2
    exit 0
fi

if [[ "$ACTION" == drop-retiring ]]; then
    # The trust registry holds at most two trusted keys and a kid is bound to one key forever. A
    # retired key that stays trusted would make the next staged rotation hit that limit after custody
    # moved. So the retiring key is revoked in the registry first (revocation is permanent), and its
    # files are destroyed only once the registry no longer trusts it. Nothing is destroyed otherwise.
    retiring_kid="$(run_key_tool show | sed -n 's/^retiring_key_id=//p')"
    if [[ -n "$retiring_kid" ]]; then
        revoke_container="$(platform_containers | head -n 1)"
        [[ -n "$revoke_container" ]] || { echo "drop-retiring needs a running Platform to revoke the retiring key in the trust registry first; nothing was removed." >&2; exit 1; }
        if ! docker exec "$revoke_container" /usr/local/bin/oteryn-platform-entrypoint php -r '
            require "vendor/autoload.php";
            $app = require "bootstrap/app.php";
            $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
            if (! app()->environment(["testing", "preproduction"])) { exit(3); }
            $kid = $argv[1] ?? "";
            $issuer = App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_ISSUER;
            $profile = App\GameAuth\NativeEvidence\NativeEvidenceContract::FRESH_PROFILE;
            $purpose = (string) config("game-auth.native_evidence.fresh_key_purpose");
            try {
                $trust = app(App\GameAuth\NativeEvidence\NativeSigningTrustRegistry::class);
                if ($trust->trustedPublicKey($issuer, $profile, $purpose, $kid) !== null) {
                    $trust->revokeKey($issuer, $profile, $purpose, $kid);
                }
                exit($trust->trustedPublicKey($issuer, $profile, $purpose, $kid) === null ? 0 : 1);
            } catch (Throwable $e) {
                exit(1);
            }
        ' -- "$retiring_kid" >/dev/null 2>&1; then
            echo "Could not revoke the retiring key in the Platform trust registry; the retiring key was kept." >&2
            exit 1
        fi
        echo "Retiring key revoked in the Platform trust registry." >&2
    fi
fi

if [[ "$ACTION" == check-kid-history ]]; then
    run_key_tool "$ACTION" "${@:2}"
    exit $?
fi

run_key_tool "$ACTION"

if [[ "$ACTION" == rotate || "$ACTION" == drop-retiring ]]; then
    while IFS= read -r container; do
        [[ -n "$container" ]] && docker restart "$container" >/dev/null
    done < <(platform_containers)
    echo "Platform restarted so it loads the new key roles; run '$0 check' once the new public key is trusted." >&2
fi
