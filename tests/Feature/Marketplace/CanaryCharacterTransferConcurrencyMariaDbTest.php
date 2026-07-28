<?php

namespace Tests\Feature\Marketplace;

use App\CanaryIntegration\CanaryCharacterTransfer;
use App\Marketplace\Data\CharacterTransferResult;
use App\Marketplace\Exceptions\MarketplaceException;
use Illuminate\Support\Facades\DB;
use PDO;
use PDOStatement;
use Tests\TestCase;

final class CanaryCharacterTransferConcurrencyMariaDbTest extends TestCase
{
    private const DATABASE = 'oteryn_canary_transfer_concurrency_test';

    private const USER = 'oteryn_transfer_concurrency_test';

    private const PASSWORD = 'oteryn-transfer-concurrency-test-password';

    private ?PDO $root = null;

    private string $rootHost = '';

    private string $rootPort = '3306';

    private string $rootPassword = '';

    protected function setUp(): void
    {
        parent::setUp();

        if (! function_exists('pcntl_fork')) {
            $this->markTestSkipped('The pcntl extension is required for the MariaDB concurrency test.');
        }

        $host = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_HOST');
        if (! is_string($host) || $host === '') {
            $this->markTestSkipped('MariaDB character-transfer integration environment is not configured.');
        }

        $port = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_PORT');
        $rootPassword = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_ROOT_PASSWORD');
        $this->rootHost = $host;
        $this->rootPort = is_string($port) && $port !== '' ? $port : '3306';
        $this->rootPassword = is_string($rootPassword) ? $rootPassword : '';
        $this->connectRoot();
        $this->resetDatabase();
        $this->configureConnection();
        config()->set('marketplace.character_limit', 10);
    }

    protected function tearDown(): void
    {
        DB::purge(CanaryCharacterTransfer::CONNECTION);

        if (! $this->root instanceof PDO && $this->rootHost !== '') {
            $this->connectRoot();
        }

        if ($this->root instanceof PDO) {
            $this->root->exec('DROP DATABASE IF EXISTS `'.self::DATABASE.'`');
            $this->root->exec("DROP USER IF EXISTS '".self::USER."'@'%'");
        }

        parent::tearDown();
    }

    public function test_competing_transfers_serialize_and_only_one_target_wins(): void
    {
        $this->insertAccount(1001);
        $this->insertAccount(1002);
        $this->insertAccount(1003);

        for ($iteration = 1; $iteration <= 3; $iteration++) {
            $playerId = $this->insertPlayer(1001);
            $prefix = sys_get_temp_dir().'/oteryn-transfer-race-'.bin2hex(random_bytes(8));
            $goPath = $prefix.'.go';
            $pids = [];
            $resultPaths = [];

            foreach ([1002, 1003] as $targetAccountId) {
                $readyPath = $prefix.'.'.$targetAccountId.'.ready';
                $resultPath = $prefix.'.'.$targetAccountId.'.json';
                $resultPaths[] = $resultPath;
                $pid = pcntl_fork();

                if ($pid === -1) {
                    self::fail('Unable to fork the MariaDB transfer concurrency worker.');
                }

                if ($pid === 0) {
                    DB::purge(CanaryCharacterTransfer::CONNECTION);
                    touch($readyPath);
                    $deadline = microtime(true) + 10;
                    while (! is_file($goPath) && microtime(true) < $deadline) {
                        usleep(10_000);
                    }

                    $result = ['status' => 'barrier_timeout'];
                    if (is_file($goPath)) {
                        try {
                            $transfer = (new CanaryCharacterTransfer)->transfer(
                                $playerId,
                                1001,
                                $targetAccountId,
                                true,
                            );
                            $result = ['status' => $transfer->status];
                        } catch (MarketplaceException $exception) {
                            $result = ['status' => $exception->reason];
                        }
                    }

                    file_put_contents($resultPath, json_encode($result, JSON_THROW_ON_ERROR));
                    exit(0);
                }

                $pids[] = $pid;
            }

            $deadline = microtime(true) + 10;
            while (microtime(true) < $deadline) {
                $ready = is_file($prefix.'.1002.ready') && is_file($prefix.'.1003.ready');
                if ($ready) {
                    break;
                }
                usleep(10_000);
            }
            self::assertFileExists($prefix.'.1002.ready');
            self::assertFileExists($prefix.'.1003.ready');
            touch($goPath);

            foreach ($pids as $pid) {
                $status = 0;
                $waitedPid = pcntl_waitpid($pid, $status);
                self::assertSame($pid, $waitedPid);
                self::assertTrue(pcntl_wifexited($status));
                self::assertSame(0, pcntl_wexitstatus($status));
            }

            $statuses = [];
            foreach ($resultPaths as $resultPath) {
                self::assertFileExists($resultPath);
                $contents = file_get_contents($resultPath);
                self::assertIsString($contents);
                $decoded = json_decode($contents, true, flags: JSON_THROW_ON_ERROR);
                self::assertIsArray($decoded);
                $status = $decoded['status'] ?? null;
                self::assertIsString($status);
                $statuses[] = $status;
            }

            sort($statuses, SORT_STRING);
            self::assertSame([
                'ownership_conflict',
                CharacterTransferResult::TRANSFERRED,
            ], $statuses);
            self::assertContains($this->playerOwner($playerId), [1002, 1003]);

            foreach (glob($prefix.'.*') ?: [] as $path) {
                @unlink($path);
            }
        }
    }

    private function resetDatabase(): void
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $this->root->exec('DROP DATABASE IF EXISTS `'.self::DATABASE.'`');
        $this->root->exec('CREATE DATABASE `'.self::DATABASE.'` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci');
        $this->root->exec("DROP USER IF EXISTS '".self::USER."'@'%'");
        $this->root->exec("CREATE USER '".self::USER."'@'%' IDENTIFIED BY '".self::PASSWORD."'");
        $this->root->exec(
            'CREATE TABLE `'.self::DATABASE.'`.`accounts` ('
            .'`id` int(11) UNSIGNED NOT NULL, '
            .'PRIMARY KEY (`id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );
        $this->root->exec(
            'CREATE TABLE `'.self::DATABASE.'`.`players` ('
            .'`id` int(11) NOT NULL AUTO_INCREMENT, '
            .'`account_id` int(11) UNSIGNED NOT NULL, '
            .'`deletion` bigint(15) NOT NULL DEFAULT 0, '
            .'PRIMARY KEY (`id`), '
            .'KEY `account_id` (`account_id`), '
            .'CONSTRAINT `players_account_fk` FOREIGN KEY (`account_id`) REFERENCES `'.self::DATABASE.'`.`accounts` (`id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );
        $this->root->exec(
            'CREATE TABLE `'.self::DATABASE.'`.`cluster_sessions` ('
            .'`account_id` int(11) UNSIGNED NOT NULL, '
            .'`player_id` int(11) NOT NULL, '
            .'`status` varchar(20) NOT NULL, '
            .'`expires_at` bigint(20) NOT NULL DEFAULT 0, '
            .'PRIMARY KEY (`account_id`), '
            .'UNIQUE KEY `player_unique` (`player_id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );

        $this->root->exec(
            'GRANT SELECT (`id`) ON `'.self::DATABASE.'`.`accounts` TO \''.self::USER.'\'@\'%\'',
        );
        $this->root->exec(
            'GRANT SELECT (`id`, `account_id`, `deletion`), UPDATE (`account_id`) ON `'.self::DATABASE.'`.`players` TO \''.self::USER.'\'@\'%\'',
        );
        $this->root->exec(
            'GRANT SELECT (`player_id`) ON `'.self::DATABASE.'`.`cluster_sessions` TO \''.self::USER.'\'@\'%\'',
        );
    }

    private function configureConnection(): void
    {
        config()->set('database.connections.'.CanaryCharacterTransfer::CONNECTION, [
            'driver' => 'mysql',
            'host' => $this->rootHost,
            'port' => $this->rootPort,
            'database' => self::DATABASE,
            'username' => self::USER,
            'password' => self::PASSWORD,
            'charset' => 'utf8mb4',
            'collation' => 'utf8mb4_unicode_ci',
            'prefix' => '',
            'prefix_indexes' => true,
            'strict' => true,
            'engine' => null,
        ]);
        DB::purge(CanaryCharacterTransfer::CONNECTION);
    }

    private function insertAccount(int $accountId): void
    {
        $this->rootExec('INSERT INTO `'.self::DATABASE.'`.`accounts` (`id`) VALUES ('.$accountId.')');
    }

    private function insertPlayer(int $accountId): int
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $this->root->exec(
            'INSERT INTO `'.self::DATABASE.'`.`players` (`account_id`) VALUES ('.$accountId.')',
        );

        return (int) $this->root->lastInsertId();
    }

    private function playerOwner(int $playerId): int
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $statement = $this->root->query(
            'SELECT `account_id` FROM `'.self::DATABASE.'`.`players` WHERE `id` = '.$playerId,
        );
        if (! $statement instanceof PDOStatement) {
            self::fail('MariaDB player owner query failed.');
        }

        $owner = $statement->fetchColumn();
        if (! is_int($owner) && ! is_string($owner)) {
            self::fail('MariaDB player owner query returned invalid data.');
        }

        return (int) $owner;
    }

    private function connectRoot(): void
    {
        $this->root = new PDO(
            "mysql:host={$this->rootHost};port={$this->rootPort};charset=utf8mb4",
            'root',
            $this->rootPassword,
            [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION],
        );
    }

    private function rootExec(string $query): void
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $this->root->exec($query);
    }
}
