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
 * transaction is open, and the default connection is the allowlisted deployed MariaDB store: driver
 * `mysql`, host, port and database name from the constants below, no unix socket and no read/write
 * split. The allowlist is code, not configuration, so an environment variable cannot widen it.
 */
final class PreproductionNativeRouteStore
{
    /** The Compose service name of the Synology preproduction MariaDB (deploy/synology/compose.yml). */
    public const ALLOWED_HOSTS = ['mariadb'];

    /** The Platform database of the Synology preproduction stack (PLATFORM_DB_NAME). */
    public const ALLOWED_DATABASES = ['oteryn_platform'];

    public const ALLOWED_PORT = 3306;

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
        $port = $connection->getConfig('port');
        if ($connection->getDriverName() !== 'mysql'
            || ! in_array($connection->getConfig('host'), self::ALLOWED_HOSTS, true)
            || ! in_array($connection->getDatabaseName(), self::ALLOWED_DATABASES, true)
            || ! in_array($connection->getConfig('unix_socket'), [null, ''], true)
            || ! in_array($port, [self::ALLOWED_PORT, (string) self::ALLOWED_PORT], true)
            || ! is_array($raw) || array_key_exists('read', $raw) || array_key_exists('write', $raw)) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::DATABASE,
                'The default database is not the allowlisted preproduction store.',
            );
        }

        return [$connection, $worldId];
    }
}
