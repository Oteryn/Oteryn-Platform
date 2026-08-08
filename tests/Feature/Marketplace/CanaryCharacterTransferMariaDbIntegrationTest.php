<?php

namespace Tests\Feature\Marketplace;

use App\CanaryIntegration\CanaryCharacterTransfer;
use App\CanaryIntegration\CanaryCharacterTransferDatabasePrivilegeVerifier;
use App\Marketplace\Data\CharacterTransferResult;
use App\Marketplace\Exceptions\MarketplaceException;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use PDO;
use PDOStatement;
use Tests\TestCase;

final class CanaryCharacterTransferMariaDbIntegrationTest extends TestCase
{
    private const DATABASE = 'oteryn_canary_character_transfer_test';

    private const USER = 'oteryn_character_transfer_test';

    private const PASSWORD = 'oteryn-character-transfer-test-password';

    private ?PDO $root = null;

    private string $rootHost = '';

    private string $rootPort = '3306';

    private string $rootPassword = '';

    protected function setUp(): void
    {
        parent::setUp();

        $host = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_HOST');
        if (! is_string($host) || $host === '') {
            $this->markTestSkipped('MariaDB character-transfer integration environment is not configured.');
        }

        $portValue = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_PORT');
        $rootPasswordValue = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_ROOT_PASSWORD');
        $this->rootHost = $host;
        $this->rootPort = is_string($portValue) && $portValue !== '' ? $portValue : '3306';
        $this->rootPassword = is_string($rootPasswordValue) ? $rootPasswordValue : '';
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

    public function test_exact_grants_snapshot_transfer_idempotency_session_and_quota_match_contract(): void
    {
        self::assertSame([], (new CanaryCharacterTransferDatabasePrivilegeVerifier)->inspect());
        $this->insertAccount(1001);
        $this->insertAccount(1002);
        $this->insertAccount(9999);
        $playerId = $this->insertPlayer(1001, 'Transfer Hero');
        $transfer = new CanaryCharacterTransfer;

        $snapshot = $transfer->snapshotOwnedCharacter(1001, $playerId);
        self::assertSame('Transfer Hero', $snapshot->name);
        self::assertArrayNotHasKey('account_id', $snapshot->publicData);
        self::assertArrayNotHasKey('deletion', $snapshot->publicData);

        $escrowed = $transfer->transfer($playerId, 1001, 9999, false);
        self::assertSame(CharacterTransferResult::TRANSFERRED, $escrowed->status);
        self::assertSame(9999, $this->playerOwner($playerId));

        $idempotent = $transfer->transfer($playerId, 1001, 9999, false);
        self::assertSame(CharacterTransferResult::ALREADY_TRANSFERRED, $idempotent->status);

        $this->rootExec(
            'INSERT INTO `'.self::DATABASE.'`.`cluster_sessions` (`account_id`, `player_id`, `status`, `expires_at`) '
            ."VALUES (9999, {$playerId}, 'ONLINE', 9999999999999)",
        );

        try {
            $transfer->transfer($playerId, 9999, 1002, true);
            self::fail('A character with a cluster session must not transfer.');
        } catch (MarketplaceException $exception) {
            self::assertSame('character_online_or_session_active', $exception->reason);
        }

        $this->rootExec('DELETE FROM `'.self::DATABASE.'`.`cluster_sessions` WHERE `player_id` = '.$playerId);
        for ($index = 1; $index <= 10; $index++) {
            $this->insertPlayer(1002, 'Target '.$index);
        }

        try {
            $transfer->transfer($playerId, 9999, 1002, true);
            self::fail('A full target account must reject transfer.');
        } catch (MarketplaceException $exception) {
            self::assertSame('target_character_limit', $exception->reason);
        }

        $this->rootExec('DELETE FROM `'.self::DATABASE.'`.`players` WHERE `name` = \'Target 10\'');
        $settled = $transfer->transfer($playerId, 9999, 1002, true);
        self::assertSame(CharacterTransferResult::TRANSFERRED, $settled->status);
        self::assertSame(1002, $this->playerOwner($playerId));
    }

    public function test_transfer_principal_is_denied_credentials_unapproved_player_updates_and_session_writes(): void
    {
        $this->insertAccount(1001);
        $playerId = $this->insertPlayer(1001, 'Denied Hero');

        $this->assertPrincipalDenied(fn () => DB::connection(CanaryCharacterTransfer::CONNECTION)
            ->table('accounts')->select('password')->where('id', 1001)->first());
        $this->assertPrincipalDenied(fn () => DB::connection(CanaryCharacterTransfer::CONNECTION)
            ->table('players')->where('id', $playerId)->update(['name' => 'Changed Hero']));
        $this->assertPrincipalDenied(fn () => DB::connection(CanaryCharacterTransfer::CONNECTION)
            ->table('players')->where('id', $playerId)->delete());
        $this->assertPrincipalDenied(fn () => DB::connection(CanaryCharacterTransfer::CONNECTION)
            ->table('cluster_sessions')->insert([
                'account_id' => 1001,
                'player_id' => $playerId,
                'status' => 'ONLINE',
                'expires_at' => 1,
            ]));
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
            .'`id` int(11) UNSIGNED NOT NULL,'
            .'`password` varchar(255) NOT NULL DEFAULT \'\','
            .'PRIMARY KEY (`id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );
        $this->root->exec($this->playersTableSql());
        $this->root->exec(
            'CREATE TABLE `'.self::DATABASE.'`.`cluster_sessions` ('
            .'`account_id` int(11) UNSIGNED NOT NULL,'
            .'`player_id` int(11) NOT NULL,'
            ."`status` enum('ACQUIRING','ONLINE','SAVING','DIRTY','OFFLINE') NOT NULL DEFAULT 'ACQUIRING',"
            .'`expires_at` bigint(20) NOT NULL DEFAULT 0,'
            .'PRIMARY KEY (`account_id`),'
            .'UNIQUE KEY `player_unique` (`player_id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );

        $playerColumns = implode(', ', array_map(
            static fn (string $column): string => "`{$column}`",
            CanaryCharacterTransfer::PLAYER_SELECT_COLUMNS,
        ));
        $this->root->exec(
            'GRANT SELECT (`id`) ON `'.self::DATABASE.'`.`accounts` TO \''.self::USER.'\'@\'%\'',
        );
        $this->root->exec(
            'GRANT SELECT ('.$playerColumns.'), UPDATE (`account_id`) ON `'.self::DATABASE.'`.`players` TO \''.self::USER.'\'@\'%\'',
        );
        $this->root->exec(
            'GRANT SELECT (`player_id`, `account_id`, `status`, `expires_at`) ON `'.self::DATABASE.'`.`cluster_sessions` TO \''.self::USER.'\'@\'%\'',
        );
    }

    private function playersTableSql(): string
    {
        return 'CREATE TABLE `'.self::DATABASE.'`.`players` ('
            .'`id` int(11) NOT NULL AUTO_INCREMENT,'
            .'`name` varchar(255) NOT NULL,'
            .'`account_id` int(11) UNSIGNED NOT NULL,'
            .'`deletion` bigint(15) NOT NULL DEFAULT 0,'
            .'`level` int(11) NOT NULL DEFAULT 100,'
            .'`vocation` int(11) NOT NULL DEFAULT 4,'
            .'`experience` bigint(20) NOT NULL DEFAULT 1000000,'
            .'`sex` int(11) NOT NULL DEFAULT 1,'
            .'`maglevel` int(11) NOT NULL DEFAULT 10,'
            .'`skill_fist` int(11) NOT NULL DEFAULT 10,'
            .'`skill_club` int(11) NOT NULL DEFAULT 10,'
            .'`skill_sword` int(11) NOT NULL DEFAULT 100,'
            .'`skill_axe` int(11) NOT NULL DEFAULT 10,'
            .'`skill_dist` int(11) NOT NULL DEFAULT 10,'
            .'`skill_shielding` int(11) NOT NULL DEFAULT 90,'
            .'`skill_fishing` int(11) NOT NULL DEFAULT 10,'
            .'`looktype` int(11) NOT NULL DEFAULT 128,'
            .'`lookaddons` int(11) NOT NULL DEFAULT 0,'
            .'`lookhead` int(11) NOT NULL DEFAULT 1,'
            .'`lookbody` int(11) NOT NULL DEFAULT 2,'
            .'`looklegs` int(11) NOT NULL DEFAULT 3,'
            .'`lookfeet` int(11) NOT NULL DEFAULT 4,'
            .'`town_id` int(11) NOT NULL DEFAULT 8,'
            .'`lastlogin` bigint(20) NOT NULL DEFAULT 0,'
            .'`lastlogout` bigint(20) NOT NULL DEFAULT 0,'
            .'PRIMARY KEY (`id`),'
            .'UNIQUE KEY `name_unique` (`name`),'
            .'KEY `account_id` (`account_id`),'
            .'CONSTRAINT `players_account_fk` FOREIGN KEY (`account_id`) REFERENCES `'.self::DATABASE.'`.`accounts` (`id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4';
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
        $this->rootExec('INSERT INTO `'.self::DATABASE.'`.`accounts` (`id`, `password`) VALUES ('.$accountId.', \'hash\')');
    }

    private function insertPlayer(int $accountId, string $name): int
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $statement = $this->root->prepare(
            'INSERT INTO `'.self::DATABASE.'`.`players` (`name`, `account_id`) VALUES (?, ?)',
        );
        $statement->execute([$name, $accountId]);

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
            self::fail('MariaDB player owner query could not be prepared.');
        }

        $value = $statement->fetchColumn();
        if (! is_int($value) && ! is_string($value)) {
            self::fail('MariaDB player owner query returned an invalid value.');
        }

        return (int) $value;
    }

    /** @param  callable(): mixed  $operation */
    private function assertPrincipalDenied(callable $operation): void
    {
        try {
            $operation();
            self::fail('The dedicated character-transfer principal unexpectedly exceeded its privileges.');
        } catch (QueryException) {
            self::addToAssertionCount(1);
        }
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
