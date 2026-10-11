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
# Usage: generate-native-admission-key.sh [ensure|rotate|drop-retiring|show]
#   ensure         create the current key if none exists; otherwise change nothing (idempotent)
#   rotate         current -> retiring, then a new current with a never-used key id
#   drop-retiring  remove the retiring key once every attempt it signed has passed exp
#   show           print the key ids and public keys, changing nothing
#
# The Platform image comes from PLATFORM_IMAGE (or OTERYN_ENV_FILE). Rotation order follows the
# native login contract 9.3: rotate, publish the new public key to the Platform trust registry and
# Game's trust config, wait for trust to propagate, and only then drop-retiring after the retiring
# key's grants have expired.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
VOLUME_NAME="oteryn-native-admission-keys"
ACTION="${1:-ensure}"

case "$ACTION" in
    ensure | rotate | drop-retiring | show) ;;
    *)
        echo "Usage: $0 [ensure|rotate|drop-retiring|show]" >&2
        exit 2
        ;;
esac

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
    $haveKey = is_file($keyFile);
    $haveKid = is_file($kidFile);
    if (! $haveKey && ! $haveKid) {
        return null;
    }
    if (! $haveKey || ! $haveKid || is_link($keyFile) || is_link($kidFile)) {
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

function usedKids(string $dir): array
{
    $file = $dir . '/used.kids';

    return is_file($file) ? array_filter(array_map('trim', file($file, FILE_IGNORE_NEW_LINES) ?: [])) : [];
}

function generate(string $dir, string $role, int $uid, int $gid): void
{
    $used = usedKids($dir);
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
    $retiring = load($dir, 'retiring', $uid);
    if ($retiring !== null) {
        echo 'retiring_key_id=' . $retiring[0] . "\n";
        echo 'retiring_public_key=' . $retiring[1] . "\n";
    }
}

if (! is_dir($dir)) {
    fail('The key volume is not mounted.');
}
secure($dir, $uid, $gid, 0700);

switch ($action) {
    case 'ensure':
        if (load($dir, 'current', $uid) === null) {
            generate($dir, 'current', $uid, $gid);
            fwrite(STDERR, "Generated the native admission signing key.\n");
        }
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

exec docker run --rm \
    --user 0:0 \
    --network none \
    --read-only \
    --cap-drop ALL --cap-add CHOWN --cap-add FOWNER --cap-add DAC_OVERRIDE \
    --security-opt no-new-privileges \
    --entrypoint php \
    --volume "$VOLUME_NAME:/keys" \
    "$PLATFORM_IMAGE" -d display_errors=stderr -r "$PHP_TOOL" -- "$ACTION"
