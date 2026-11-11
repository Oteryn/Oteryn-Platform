<?php

namespace Tests\Feature\GameAuth\NativeTopology;

use App\GameAuth\Worlds\DisposableNativeStore;
use Illuminate\Database\Connection;
use Illuminate\Support\Facades\DB;
use LogicException;
use Tests\TestCase;

/**
 * Owner amendment D1023 (ADR 0044, ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2): `DisposableNativeStore` admits
 * one PostgreSQL profile, the test suite's own `oteryn_test` database on a loopback TCP host in `testing`.
 * Every other host, database, environment or read/write split refuses, the operator commands still refuse
 * PostgreSQL, and no refusal names a credential or host.
 *
 * Connections are lazy, so the guard is exercised on a configured probe connection without a PostgreSQL
 * server; the test runs unchanged on every suite engine.
 */
final class DisposableNativeStorePostgresProfileTest extends TestCase
{
    private const PROBE = 'disposable_pgsql_probe';

    private const PASSWORD = 'placeholder-not-a-secret';

    private const PROFILE = [
        'driver' => 'pgsql',
        'host' => '127.0.0.1',
        'port' => '5432',
        'database' => 'oteryn_test',
        'username' => 'oteryn_probe_user',
        'password' => self::PASSWORD,
        'charset' => 'utf8',
        'prefix' => '',
        'prefix_indexes' => true,
        'search_path' => 'public',
        'sslmode' => 'prefer',
    ];

    private string $originalDefault = '';

    protected function setUp(): void
    {
        parent::setUp();
        $this->originalDefault = DB::getDefaultConnection();
    }

    protected function tearDown(): void
    {
        if ($this->app !== null) {
            $this->app->detectEnvironment(fn (): string => 'testing');
            DB::setDefaultConnection($this->originalDefault);
            DB::purge(self::PROBE);
        }
        parent::tearDown();
    }

    public function test_the_loopback_oteryn_test_database_is_admitted_in_testing_only(): void
    {
        foreach (['127.0.0.1', 'localhost'] as $host) {
            $this->probe(['host' => $host]);
            $connection = (new DisposableNativeStore)->connection();
            self::assertSame(self::PROBE, $connection->getName());
            self::assertSame('pgsql', $connection->getDriverName());
            self::assertSame('oteryn_test', $connection->getDatabaseName());
        }
    }

    public function test_any_non_loopback_or_non_tcp_host_refuses(): void
    {
        foreach (['::1', 'postgres', 'mariadb', 'db.example.test', '10.0.0.5', '127.0.0.2', '0.0.0.0',
            '/var/run/postgresql', '', ' 127.0.0.1', '127.0.0.1 ', 'LOCALHOST', 'localhost.', null,
            ['127.0.0.1'], ['127.0.0.1', '10.0.0.5']] as $host) {
            $this->probe(['host' => $host]);
            $this->assertRefused(var_export($host, true));
        }
    }

    public function test_any_database_other_than_oteryn_test_refuses(): void
    {
        foreach (['oteryn', 'oteryn_platform', 'oteryn_concurrency', 'postgres', 'template1',
            'OTERYN_TEST', 'oteryn_test ', ' oteryn_test', 'oteryn_test2', ''] as $database) {
            $this->probe(['database' => $database]);
            $this->assertRefused($database);
        }
    }

    public function test_any_environment_other_than_testing_refuses(): void
    {
        foreach (['preproduction', 'production', 'staging', 'local', 'development', 'testing-preproduction'] as $environment) {
            $this->probe();
            $this->app->detectEnvironment(fn (): string => $environment);
            try {
                $this->assertRefused($environment);
            } finally {
                $this->app->detectEnvironment(fn (): string => 'testing');
            }
        }
    }

    public function test_a_read_or_write_split_refuses(): void
    {
        foreach ([
            ['read' => ['host' => '10.0.0.9'], 'write' => ['host' => '127.0.0.1']],
            ['read' => ['database' => 'oteryn_platform'], 'write' => []],
            ['read' => ['host' => '127.0.0.1'], 'write' => ['host' => '127.0.0.1']],
        ] as $index => $split) {
            $this->probe($split);
            $this->assertRefused('split '.$index);
        }
    }

    public function test_operator_commands_still_refuse_the_admitted_postgres_profile(): void
    {
        $this->probe();
        self::assertInstanceOf(Connection::class, (new DisposableNativeStore)->connection());
        try {
            (new DisposableNativeStore)->retainedRun();
            self::fail('The operator commands must refuse PostgreSQL.');
        } catch (LogicException $exception) {
            self::assertSame('Native operator commands require the retained per-run SQLite file.', $exception->getMessage());
        }
    }

    /** @param array<string, mixed> $override */
    private function probe(array $override = []): void
    {
        config(['database.connections.'.self::PROBE => array_merge(self::PROFILE, $override)]);
        DB::purge(self::PROBE);
        DB::setDefaultConnection(self::PROBE);
    }

    private function assertRefused(string $case): void
    {
        try {
            (new DisposableNativeStore)->connection();
            self::fail('The disposable store must refuse: '.$case);
        } catch (LogicException $exception) {
            $message = $exception->getMessage();
            self::assertNotSame('', $message, $case);
            self::assertStringNotContainsString(self::PASSWORD, $message, $case);
            self::assertStringNotContainsString('oteryn_probe_user', $message, $case);
            self::assertStringNotContainsString('10.0.0', $message, $case);
        }
    }
}
