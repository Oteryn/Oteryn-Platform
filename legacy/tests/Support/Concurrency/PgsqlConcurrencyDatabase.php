<?php

namespace Tests\Support\Concurrency;

use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use PDO;

/**
 * Provisions a throwaway PostgreSQL database for multi-process concurrency tests.
 *
 * The tests need committed data that several OS processes can see, so they cannot use the
 * transaction-per-test isolation of RefreshDatabase. Each test class creates its own database,
 * migrates it, points the "pgsql" connection at it and drops it again afterwards.
 *
 * Enabled only when PGSQL_INTEGRATION_HOST is set (the CI "platform-postgres" job sets it).
 */
final class PgsqlConcurrencyDatabase
{
    private string $host = '';

    private string $port = '5432';

    private string $user = '';

    private string $password = '';

    private string $database = '';

    /** @return string|null The reason the environment cannot run these tests, or null when it can. */
    public static function unavailableReason(): ?string
    {
        if (! function_exists('pcntl_fork')) {
            return 'The pcntl extension is required for multi-process concurrency tests.';
        }
        if (! extension_loaded('pdo_pgsql')) {
            return 'The pdo_pgsql extension is required for PostgreSQL concurrency tests.';
        }
        $host = getenv('PGSQL_INTEGRATION_HOST');
        if (! is_string($host) || $host === '') {
            return 'PostgreSQL integration environment is not configured.';
        }

        return null;
    }

    public function create(string $database): void
    {
        $host = getenv('PGSQL_INTEGRATION_HOST');
        $port = getenv('PGSQL_INTEGRATION_PORT');
        $user = getenv('PGSQL_INTEGRATION_USER');
        $password = getenv('PGSQL_INTEGRATION_PASSWORD');

        $this->host = is_string($host) ? $host : '127.0.0.1';
        $this->port = is_string($port) && $port !== '' ? $port : '5432';
        $this->user = is_string($user) && $user !== '' ? $user : 'postgres';
        $this->password = is_string($password) ? $password : '';
        $this->database = $database;

        $admin = $this->admin();
        $admin->exec('DROP DATABASE IF EXISTS "'.$database.'"');
        $admin->exec('CREATE DATABASE "'.$database.'" ENCODING \'UTF8\'');
        unset($admin);

        config()->set('database.default', 'pgsql');
        config()->set('database.connections.pgsql', [
            'driver' => 'pgsql',
            'host' => $this->host,
            'port' => $this->port,
            'database' => $database,
            'username' => $this->user,
            'password' => $this->password,
            'charset' => 'utf8',
            'prefix' => '',
            'prefix_indexes' => true,
            'search_path' => 'public',
            'sslmode' => 'prefer',
            'timezone' => 'UTC',
        ]);
        DB::purge('pgsql');

        $status = Artisan::call('migrate:fresh', [
            '--database' => 'pgsql',
            '--force' => true,
            '--no-interaction' => true,
        ]);
        if ($status !== 0) {
            throw new \RuntimeException('migrate:fresh failed on PostgreSQL: '.trim(Artisan::output()));
        }
    }

    public function drop(): void
    {
        DB::purge('pgsql');
        if ($this->database === '') {
            return;
        }

        $admin = $this->admin();
        $admin->exec(
            'SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = \''.$this->database.'\' AND pid <> pg_backend_pid()',
        );
        $admin->exec('DROP DATABASE IF EXISTS "'.$this->database.'"');
        $this->database = '';
    }

    /**
     * A short-lived administrative connection. It is never kept across a fork: a child process
     * that exits would close the shared socket and terminate the parent's session.
     */
    private function admin(): PDO
    {
        return new PDO(
            "pgsql:host={$this->host};port={$this->port};dbname=postgres",
            $this->user,
            $this->password,
            [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION],
        );
    }
}
