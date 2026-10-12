<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Connection;
use Illuminate\Support\Facades\DB;

/**
 * The preproduction-store fence of native topology issuance and route publication (ADR 0044, the
 * ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 amendment accepted by the owner on 2026-10-11, option a). It is a
 * separate fence beside `DisposableNativeStore`, which stays unchanged and keeps guarding every other
 * path: the disposable per-run store, readback in tests and the trust-key command.
 *
 * The mode is entered only by the explicit switch `game-auth.native_route_preproduction_store.enabled`
 * (default off). A malformed switch refuses instead of falling back to the disposable path. Once on,
 * every condition must hold or the command refuses with a named reason and nothing written:
 * `APP_ENV` is exactly `preproduction` (not `testing`), the configured native World
 * (`game-auth.native_admission.unverified_character_world_id`) is a canonical UUIDv7, no outer
 * transaction is open, and the default connection is the allowlisted deployed store of its engine,
 * with no unix socket and no read/write split.
 *
 * The store identity is keyed by the default connection's driver (ADR 0044 amendment P4, ADR 0046 §8):
 * `mysql` admits only the Synology MariaDB store (host `mariadb`, port 3306), removed in P7; `pgsql`
 * admits only the Synology PostgreSQL store (host `postgres`, port 5432) and only over TLS with
 * `sslmode=verify-full`. Both name the database `oteryn_platform`. A deployment has one default
 * connection, chosen by the engine switch `PLATFORM_DB_ENGINE`, so exactly one identity is reachable at
 * a time; a driver crossed with the other identity's host or port refuses. The allowlist is code, not
 * configuration, so an environment variable cannot widen it.
 */
final class PreproductionNativeRouteStore
{
    /** The Compose service name of the Synology preproduction MariaDB (deploy/synology/compose.yml). */
    public const ALLOWED_HOSTS = ['mariadb'];

    /** The Platform database of the Synology preproduction stack (PLATFORM_DB_NAME). */
    public const ALLOWED_DATABASES = ['oteryn_platform'];

    public const ALLOWED_PORT = 3306;

    /** The Compose service name of the Synology preproduction PostgreSQL (deploy/synology/compose.yml). */
    public const ALLOWED_PGSQL_HOSTS = ['postgres'];

    public const ALLOWED_PGSQL_PORT = 5432;

    /** The only libpq TLS mode the PostgreSQL identity admits: encrypted and the server name verified. */
    public const REQUIRED_PGSQL_SSLMODE = 'verify-full';

    /** True only when the explicit switch is on; off when unset or false; a malformed value refuses. */
    public function requested(): bool
    {
        $value = config('game-auth.native_route_preproduction_store.enabled');
        if ($value === null || $value === false || $value === '' || $value === 'false') {
            return false;
        }
        if ($value === true || $value === 'true') {
            return true;
        }

        throw new NativePreproductionStoreRefused(
            NativePreproductionStoreRefused::SWITCH_INVALID,
            'The preproduction-store switch must be true or false.',
        );
    }

    /**
     * @return array{Connection, string} the admitted default connection and the configured native WorldId
     */
    public function admit(): array
    {
        if (! $this->requested()) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::NOT_ENABLED,
                'The preproduction-store mode is not enabled.',
            );
        }
        if (! app()->environment('preproduction')) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::ENVIRONMENT,
                'The preproduction-store mode runs only with APP_ENV=preproduction.',
            );
        }

        $worldId = config('game-auth.native_admission.unverified_character_world_id');
        if (! is_string($worldId) || ! NativeTopologyReceipt::isCanonicalId($worldId)) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::WORLD_UNCONFIGURED,
                'The configured native World is not a canonical UUIDv7 WorldId.',
            );
        }

        $connection = DB::connection();
        if ($connection->transactionLevel() !== 0) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::TRANSACTION_OPEN,
                'The preproduction-store mode requires its own committed transaction boundary.',
            );
        }

        $raw = config('database.connections.'.$connection->getName());
        if (! $this->isAllowlistedStore($connection)
            || ! in_array($connection->getDatabaseName(), self::ALLOWED_DATABASES, true)
            || ! in_array($connection->getConfig('unix_socket'), [null, ''], true)
            || ! is_array($raw) || array_key_exists('read', $raw) || array_key_exists('write', $raw)) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::DATABASE,
                'The default database is not the allowlisted preproduction store.',
            );
        }

        return [$connection, $worldId];
    }

    /** The driver selects one identity; host, port and (for PostgreSQL) the TLS mode must all match it. */
    private function isAllowlistedStore(Connection $connection): bool
    {
        $host = $connection->getConfig('host');
        $port = $connection->getConfig('port');

        return match ($connection->getDriverName()) {
            'mysql' => in_array($host, self::ALLOWED_HOSTS, true)
                && in_array($port, [self::ALLOWED_PORT, (string) self::ALLOWED_PORT], true),
            'pgsql' => in_array($host, self::ALLOWED_PGSQL_HOSTS, true)
                && in_array($port, [self::ALLOWED_PGSQL_PORT, (string) self::ALLOWED_PGSQL_PORT], true)
                && $connection->getConfig('sslmode') === self::REQUIRED_PGSQL_SSLMODE,
            default => false,
        };
    }
}
