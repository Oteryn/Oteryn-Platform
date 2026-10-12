<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Connection;
use Illuminate\Support\Facades\DB;
use LogicException;

/**
 * The one disposable-store guard of native topology issuance/readback, the route command and the trust
 * command (ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2). `connection()` is the predicate moved unchanged out of
 * `NativeTopologyRegistry::isolatedConnection()`; it may not be relaxed or forked. The one exception is the
 * owner amendment D1023 (ADR 0044, "Amendment D1023"): a testing-only loopback PostgreSQL profile, fixed in
 * code below and not widened by any environment variable. The two further checks narrow it for the
 * operator commands only, which therefore still refuse PostgreSQL.
 */
final class DisposableNativeStore
{
    /** D1023: the only PostgreSQL database name admitted, and only in `testing`. */
    private const POSTGRES_TEST_DATABASE = 'oteryn_test';

    /** D1023: loopback TCP hosts only; a socket path or any other host refuses. */
    private const POSTGRES_LOOPBACK_HOSTS = ['127.0.0.1', 'localhost'];

    public function connection(): Connection
    {
        if (! app()->environment(['testing', 'preproduction'])) {
            throw new LogicException('Native topology issuance/readback is restricted to disposable testing or preproduction.');
        }

        $connection = DB::connection();
        if ($connection->transactionLevel() !== 0) {
            throw new LogicException('Native topology issuance/readback requires its own committed transaction boundary.');
        }

        $database = $connection->getDatabaseName();
        if ($connection->getDriverName() === 'mysql') {
            if ($database !== 'oteryn_concurrency'
                || ! in_array($connection->getConfig('host'), ['127.0.0.1', '::1', 'localhost'], true)
                || ! in_array($connection->getConfig('unix_socket'), [null, ''], true)
                || ! app()->environment('testing')) {
                throw new LogicException('Native topology requires the registered loopback disposable MariaDB test database.');
            }

            return $connection;
        }
        if ($connection->getDriverName() === 'pgsql') {
            // D1023: the PostgreSQL test suite's own database on loopback, in `testing` only.
            $raw = config('database.connections.'.$connection->getName());
            if ($database !== self::POSTGRES_TEST_DATABASE
                || ! in_array($connection->getConfig('host'), self::POSTGRES_LOOPBACK_HOSTS, true)
                || ! is_array($raw) || array_key_exists('read', $raw) || array_key_exists('write', $raw)
                || ! app()->environment('testing')) {
                throw new LogicException('Native topology requires the registered loopback disposable PostgreSQL test database.');
            }

            return $connection;
        }
        if ($connection->getDriverName() !== 'sqlite') {
            throw new LogicException('Unsupported disposable native topology database profile.');
        }
        if ($database === ':memory:' && app()->environment('testing')) {
            return $connection;
        }

        // Check before opening the issuer connection: the qualifying runner owns
        // this retained disposable directory. These guards do not prove custody.
        $temporaryRoot = realpath(sys_get_temp_dir());
        $directory = realpath(dirname($database));
        if ($temporaryRoot === false || $directory === false
            || dirname($directory) !== $temporaryRoot
            || preg_match('/\Aoteryn-native-topology-[0-9a-f]+\z/', basename($directory)) !== 1
            || $directory !== dirname($database)
            || basename($database) !== 'oteryn-native-topology.sqlite'
            || is_link($directory) || is_link($database)
            || ! is_file($database) || realpath($database) !== $database) {
            throw new LogicException('Native topology requires a retained regular SQLite fixture inside its controlled disposable directory.');
        }

        return $connection;
    }

    /**
     * The guarded connection, admitted only as the retained per-run SQLite file in `testing` as in
     * `preproduction`: `:memory:`, the loopback `oteryn_concurrency` database and the D1023 loopback
     * PostgreSQL `oteryn_test` database name no per-run directory.
     *
     * @return array{Connection, string} the connection and its canonical per-run directory
     */
    public function retainedRun(): array
    {
        $connection = $this->connection();
        $database = $connection->getDatabaseName();
        if ($connection->getDriverName() !== 'sqlite' || $database === ':memory:'
            || basename($database) !== 'oteryn-native-topology.sqlite') {
            throw new LogicException('Native operator commands require the retained per-run SQLite file.');
        }

        return [$connection, dirname($database)];
    }

    /**
     * The configured high-water directory must be canonical, so neither it nor any parent is a symlink,
     * and sit directly beneath the per-run directory of the current database file.
     */
    public function assertHighWaterDirectoryWithin(string $runDirectory): void
    {
        $configured = config('game-auth.native_evidence.high_water_directory');
        if (! is_string($configured) || $configured === '') {
            throw new LogicException('Native trust publication requires a per-run high-water directory.');
        }

        $directory = realpath($configured);
        if ($directory === false || $directory !== $configured
            || is_link($configured) || ! is_dir($directory)
            || dirname($directory) !== $runDirectory || realpath($runDirectory) !== $runDirectory) {
            throw new LogicException('Native trust publication requires a canonical high-water directory inside the current run directory.');
        }
    }
}
