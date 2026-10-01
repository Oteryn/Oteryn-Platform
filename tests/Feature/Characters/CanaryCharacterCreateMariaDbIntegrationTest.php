<?php

namespace Tests\Feature\Characters;

use App\CanaryIntegration\CanaryCharacterCreateDatabasePrivilegeVerifier;
use App\CanaryIntegration\CanaryCharacterCreator;
use App\Characters\Exceptions\CharacterAccountMissing;
use App\Characters\Exceptions\CharacterLimitReached;
use App\Characters\Exceptions\CharacterNameConflict;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use PDO;
use Tests\TestCase;

final class CanaryCharacterCreateMariaDbIntegrationTest extends TestCase
{
    private const DATABASE = 'oteryn_canary_character_create_test';

    private const USER = 'oteryn_character_create_test';

    private const PASSWORD = 'oteryn-character-create-test-password';

    private ?PDO $root = null;

    private string $rootHost = '';

    private string $rootPort = '3306';

    private string $rootPassword = '';

    protected function setUp(): void
    {
        parent::setUp();

        $host = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_HOST');

        if (! is_string($host) || $host === '') {
            $this->markTestSkipped('MariaDB character-create integration environment is not configured.');
        }

        $portValue = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_PORT');
        $rootPasswordValue = getenv('CANARY_CHARACTER_CREATE_INTEGRATION_ROOT_PASSWORD');
        $port = is_string($portValue) && $portValue !== '' ? $portValue : '3306';
        $rootPassword = is_string($rootPasswordValue) ? $rootPasswordValue : '';

        $this->rootHost = $host;
        $this->rootPort = $port;
        $this->rootPassword = $rootPassword;
        $this->connectRoot();

        $this->resetDatabase();
        $this->configureCharacterCreateConnection($host, $port);
    }

    protected function tearDown(): void
    {
        DB::purge(CanaryCharacterCreator::CONNECTION);

        if (! $this->root instanceof PDO && $this->rootHost !== '') {
            $this->connectRoot();
        }

        if ($this->root instanceof PDO) {
            $this->root->exec('DROP DATABASE IF EXISTS `'.self::DATABASE.'`');
            $this->root->exec("DROP USER IF EXISTS '".self::USER."'@'%'");
        }

        parent::tearDown();
    }

    public function test_exact_grants_for_update_starter_defaults_idempotency_and_denials_match_contract(): void
    {
        self::assertSame([], (new CanaryCharacterCreateDatabasePrivilegeVerifier)->inspect());

        $this->insertAccount(1001);
        $creator = new CanaryCharacterCreator;

        $created = $creator->create(1001, 'Alice Moon', 9, 1);
        $recovered = $creator->create(1001, 'Alice Moon', 1, 0);

        self::assertTrue($created->created);
        self::assertFalse($recovered->created);
        self::assertSame($created->playerId, $recovered->playerId);
        self::assertSame('Alice Moon', $recovered->canonicalName);

        $row = $this->rootRow('SELECT * FROM `'.self::DATABASE.'`.`players` WHERE `id` = '.$created->playerId);
        self::assertSame('Alice Moon', $this->rowString($row, 'name'));
        self::assertSame('', $this->rowString($row, 'conditions'));

        $expectedIntegers = [
            'account_id' => 1001,
            'level' => 8,
            'vocation' => 9,
            'health' => 185,
            'healthmax' => 185,
            'experience' => 4200,
            'mana' => 90,
            'manamax' => 90,
            'soul' => 100,
            'cap' => 470,
            'town_id' => 8,
            'posx' => 0,
            'posy' => 0,
            'posz' => 0,
            'sex' => 1,
            'pronoun' => 0,
            'looktype' => 128,
            'skill_fist' => 10,
            'skill_fist_tries' => 0,
            'deletion' => 0,
            'save' => 1,
            'balance' => 0,
            'stamina' => 2520,
            'offlinetraining_time' => 43200,
            'offlinetraining_skill' => -1,
            'isreward' => 1,
            'forge_dust_level' => 100,
        ];

        foreach ($expectedIntegers as $column => $expected) {
            self::assertSame($expected, $this->rowInt($row, $column), $column);
        }

        $this->assertCharacterPrincipalDenied(fn () => DB::connection(CanaryCharacterCreator::CONNECTION)
            ->table('accounts')->select('password')->where('id', 1001)->first());
        $this->assertCharacterPrincipalDenied(fn () => DB::connection(CanaryCharacterCreator::CONNECTION)
            ->table('players')->select('comment')->where('id', $created->playerId)->first());
        $this->assertCharacterPrincipalDenied(fn () => DB::connection(CanaryCharacterCreator::CONNECTION)
            ->table('players')->where('id', $created->playerId)->update(['name' => 'Changed Name']));
        $this->assertCharacterPrincipalDenied(fn () => DB::connection(CanaryCharacterCreator::CONNECTION)
            ->table('players')->where('id', $created->playerId)->delete());
        $this->assertCharacterPrincipalDenied(fn () => DB::connection(CanaryCharacterCreator::CONNECTION)
            ->table('player_items')->insert(['player_id' => $created->playerId]));
    }

    public function test_missing_account_fails_closed(): void
    {
        $this->expectException(CharacterAccountMissing::class);

        (new CanaryCharacterCreator)->create(9999, 'Missing Account', 1, 0);
    }

    public function test_deleted_rows_do_not_count_but_still_reserve_their_name(): void
    {
        $this->insertAccount(1002);

        for ($index = 1; $index <= 9; $index++) {
            $this->insertExistingPlayer(1002, "Active {$index}", 0);
        }

        $this->insertExistingPlayer(1002, 'Deleted Hero', 123456);

        $created = (new CanaryCharacterCreator)->create(1002, 'Tenth Hero', 4, 0);
        self::assertTrue($created->created);
        self::assertSame(10, $this->activeCount(1002));

        try {
            (new CanaryCharacterCreator)->create(1002, 'Deleted Hero', 4, 0);
            self::fail('A soft-deleted row must continue to reserve its globally unique name.');
        } catch (CharacterNameConflict) {
            // @phpstan-ignore staticMethod.alreadyNarrowedType (re-reads the active count after a rejected create)
            self::assertSame(10, $this->activeCount(1002));
        }

        $recovered = (new CanaryCharacterCreator)->create(1002, 'Tenth Hero', 1, 1);
        self::assertFalse($recovered->created);

        $this->expectException(CharacterLimitReached::class);
        (new CanaryCharacterCreator)->create(1002, 'Eleventh Hero', 1, 1);
    }

    public function test_same_account_concurrent_last_slot_allows_exactly_one_new_character(): void
    {
        $this->requirePcntl();
        $this->insertAccount(1003);

        for ($index = 1; $index <= 9; $index++) {
            $this->insertExistingPlayer(1003, "Quota {$index}", 0);
        }

        $results = $this->runConcurrentCreates([
            ['account_id' => 1003, 'name' => 'Quota Alpha', 'vocation' => 1, 'sex' => 0],
            ['account_id' => 1003, 'name' => 'Quota Beta', 'vocation' => 2, 'sex' => 1],
        ]);

        sort($results);
        self::assertSame(['created', 'limit'], $results);
        self::assertSame(10, $this->activeCount(1003));
    }

    public function test_two_accounts_racing_same_name_commit_exactly_one_global_name(): void
    {
        $this->requirePcntl();
        $this->insertAccount(1004);
        $this->insertAccount(1005);

        $results = $this->runConcurrentCreates([
            ['account_id' => 1004, 'name' => 'Global Race', 'vocation' => 3, 'sex' => 0],
            ['account_id' => 1005, 'name' => 'Global Race', 'vocation' => 4, 'sex' => 1],
        ]);

        sort($results);
        self::assertSame(['created', 'name_conflict'], $results);
        self::assertSame(1, $this->rootCount('SELECT COUNT(*) FROM `'.self::DATABASE."`.`players` WHERE `name` = 'Global Race'"));
    }

    public function test_previously_committed_same_account_character_is_forward_recovered_without_update_privilege(): void
    {
        $this->insertAccount(1006);
        $existingId = $this->insertExistingPlayer(1006, 'Committed Hero', 0);

        $result = (new CanaryCharacterCreator)->create(1006, 'Committed Hero', 9, 1);

        self::assertFalse($result->created);
        self::assertSame($existingId, $result->playerId);
        self::assertSame(1, $this->rootCount('SELECT COUNT(*) FROM `'.self::DATABASE."`.`players` WHERE `name` = 'Committed Hero'"));
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
            .'`id` int(11) UNSIGNED NOT NULL AUTO_INCREMENT,'
            .'`name` varchar(32) NOT NULL DEFAULT \'\','
            .'`password` varchar(255) NOT NULL DEFAULT \'\','
            .'PRIMARY KEY (`id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );

        $this->root->exec($this->playersTableSql());
        $this->root->exec(
            'CREATE TABLE `'.self::DATABASE.'`.`player_items` ('
            .'`id` int(11) NOT NULL AUTO_INCREMENT,'
            .'`player_id` int(11) NOT NULL,'
            .'PRIMARY KEY (`id`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );

        $this->root->exec(
            'GRANT SELECT (`id`) ON `'.self::DATABASE.'`.`accounts` '
            ."TO '".self::USER."'@'%'",
        );
        $this->root->exec(
            'GRANT SELECT (`id`, `name`, `account_id`, `deletion`) ON `'.self::DATABASE.'`.`players` '
            ."TO '".self::USER."'@'%'",
        );

        $insertColumns = implode(', ', array_map(
            static fn (string $column): string => "`{$column}`",
            CanaryCharacterCreateDatabasePrivilegeVerifier::PLAYER_INSERT_COLUMNS,
        ));
        $this->root->exec(
            'GRANT INSERT ('.$insertColumns.') ON `'.self::DATABASE.'`.`players` '
            ."TO '".self::USER."'@'%'",
        );
    }

    private function playersTableSql(): string
    {
        return 'CREATE TABLE `'.self::DATABASE.'`.`players` ('
            .'`id` int(11) NOT NULL AUTO_INCREMENT,'
            .'`name` varchar(255) NOT NULL,'
            .'`group_id` int(11) NOT NULL DEFAULT 1,'
            .'`account_id` int(11) UNSIGNED NOT NULL,'
            .'`level` int(11) NOT NULL DEFAULT 1,'
            .'`vocation` int(11) NOT NULL DEFAULT 0,'
            .'`health` int(11) NOT NULL DEFAULT 150,'
            .'`healthmax` int(11) NOT NULL DEFAULT 150,'
            .'`experience` bigint(20) NOT NULL DEFAULT 0,'
            .'`lookbody` int(11) NOT NULL DEFAULT 0,'
            .'`lookfeet` int(11) NOT NULL DEFAULT 0,'
            .'`lookhead` int(11) NOT NULL DEFAULT 0,'
            .'`looklegs` int(11) NOT NULL DEFAULT 0,'
            .'`looktype` int(11) NOT NULL DEFAULT 136,'
            .'`lookaddons` int(11) NOT NULL DEFAULT 0,'
            .'`maglevel` int(11) NOT NULL DEFAULT 0,'
            .'`mana` int(11) NOT NULL DEFAULT 0,'
            .'`manamax` int(11) NOT NULL DEFAULT 0,'
            .'`manaspent` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`soul` int(10) UNSIGNED NOT NULL DEFAULT 0,'
            .'`town_id` int(11) NOT NULL DEFAULT 1,'
            .'`posx` int(11) NOT NULL DEFAULT 0,'
            .'`posy` int(11) NOT NULL DEFAULT 0,'
            .'`posz` int(11) NOT NULL DEFAULT 0,'
            .'`conditions` mediumblob NOT NULL,'
            .'`cap` int(11) NOT NULL DEFAULT 0,'
            .'`sex` int(11) NOT NULL DEFAULT 0,'
            .'`pronoun` int(11) NOT NULL DEFAULT 0,'
            .'`deletion` bigint(15) NOT NULL DEFAULT 0,'
            .'`save` tinyint(1) NOT NULL DEFAULT 1,'
            .'`balance` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`offlinetraining_time` smallint(5) UNSIGNED NOT NULL DEFAULT 43200,'
            .'`offlinetraining_skill` tinyint(2) NOT NULL DEFAULT -1,'
            .'`stamina` smallint(5) UNSIGNED NOT NULL DEFAULT 2520,'
            .'`skill_fist` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_fist_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`skill_club` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_club_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`skill_sword` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_sword_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`skill_axe` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_axe_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`skill_dist` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_dist_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`skill_shielding` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_shielding_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`skill_fishing` int(10) UNSIGNED NOT NULL DEFAULT 10,'
            .'`skill_fishing_tries` bigint(20) UNSIGNED NOT NULL DEFAULT 0,'
            .'`isreward` tinyint(1) NOT NULL DEFAULT 1,'
            .'`istutorial` tinyint(1) NOT NULL DEFAULT 0,'
            .'`forge_dust_level` bigint(21) NOT NULL DEFAULT 100,'
            .'`comment` varchar(255) NOT NULL DEFAULT \'\','
            .'PRIMARY KEY (`id`),'
            .'UNIQUE KEY `players_unique` (`name`),'
            .'KEY `account_id` (`account_id`),'
            .'CONSTRAINT `players_account_fk` FOREIGN KEY (`account_id`) REFERENCES `'.self::DATABASE.'`.`accounts` (`id`) ON DELETE CASCADE'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8';
    }

    private function configureCharacterCreateConnection(string $host, string $port): void
    {
        config()->set('database.connections.'.CanaryCharacterCreator::CONNECTION, [
            'driver' => 'mysql',
            'host' => $host,
            'port' => $port,
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

        DB::purge(CanaryCharacterCreator::CONNECTION);
    }

    private function insertAccount(int $accountId): void
    {
        $this->rootExec(
            'INSERT INTO `'.self::DATABASE.'`.`accounts` (`id`, `name`, `password`) '
            ."VALUES ({$accountId}, 'account{$accountId}', 'secret-hash')",
        );
    }

    private function insertExistingPlayer(int $accountId, string $name, int $deletion): int
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $statement = $this->root->prepare(
            'INSERT INTO `'.self::DATABASE.'`.`players` (`name`, `account_id`, `conditions`, `deletion`) VALUES (?, ?, ?, ?)',
        );
        $statement->execute([$name, $accountId, '', $deletion]);

        return (int) $this->root->lastInsertId();
    }

    private function activeCount(int $accountId): int
    {
        return $this->rootCount(
            'SELECT COUNT(*) FROM `'.self::DATABASE.'`.`players` WHERE `account_id` = '.$accountId.' AND `deletion` = 0',
        );
    }

    /**
     * @param  list<array{account_id: int, name: string, vocation: int, sex: int}>  $requests
     * @return list<string>
     */
    private function runConcurrentCreates(array $requests): array
    {
        $directory = sys_get_temp_dir().'/oteryn-character-race-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700, true));
        $barrier = $directory.'/go';
        $pids = [];

        // PDO/MySQL connections are not fork-safe. Disconnect the privileged root
        // socket before forking so child shutdown cannot terminate the parent's
        // inherited server connection. The parent reconnects after all workers exit.
        $this->root = null;
        DB::purge(CanaryCharacterCreator::CONNECTION);

        foreach ($requests as $index => $request) {
            $pid = pcntl_fork();

            if ($pid === -1) {
                self::fail('Unable to fork MariaDB race-test process.');
            }

            if ($pid === 0) {
                DB::purge(CanaryCharacterCreator::CONNECTION);

                while (! file_exists($barrier)) {
                    usleep(1000);
                }

                try {
                    $result = (new CanaryCharacterCreator)->create(
                        $request['account_id'],
                        $request['name'],
                        $request['vocation'],
                        $request['sex'],
                    );
                    $status = $result->created ? 'created' : 'existing';
                } catch (CharacterLimitReached) {
                    $status = 'limit';
                } catch (CharacterNameConflict) {
                    $status = 'name_conflict';
                } catch (\Throwable $exception) {
                    $status = 'unexpected:'.get_class($exception);
                }

                file_put_contents($directory.'/result-'.$index, $status);
                exit(0);
            }

            $pids[] = $pid;
        }

        touch($barrier);

        foreach ($pids as $pid) {
            $status = 0;
            $waitedPid = pcntl_waitpid($pid, $status);
            self::assertSame($pid, $waitedPid);

            if (! is_int($status)) {
                self::fail('pcntl_waitpid returned a non-integer process status.');
            }

            self::assertTrue(pcntl_wifexited($status));
            self::assertSame(0, pcntl_wexitstatus($status));
        }

        $this->connectRoot();

        $results = [];

        foreach (array_keys($requests) as $index) {
            $result = file_get_contents($directory.'/result-'.$index);
            self::assertIsString($result);
            $results[] = $result;
            unlink($directory.'/result-'.$index);
        }

        unlink($barrier);
        rmdir($directory);
        DB::purge(CanaryCharacterCreator::CONNECTION);

        return $results;
    }

    private function requirePcntl(): void
    {
        if (! function_exists('pcntl_fork')) {
            $this->markTestSkipped('pcntl is required for MariaDB race integration tests.');
        }
    }

    /**
     * @param  callable(): mixed  $operation
     */
    private function assertCharacterPrincipalDenied(callable $operation): void
    {
        try {
            $operation();
            self::fail('The dedicated character-create principal unexpectedly exceeded its approved privileges.');
        } catch (QueryException) {
            return;
        }
    }

    private function connectRoot(): void
    {
        if ($this->rootHost === '') {
            self::fail('MariaDB root connection parameters are unavailable.');
        }

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

    private function rootCount(string $query): int
    {
        $value = $this->rootValue($query);

        if (! is_int($value) && ! is_string($value)) {
            self::fail('MariaDB count query returned a non-scalar value.');
        }

        return (int) $value;
    }

    /**
     * @param  array<string, mixed>  $row
     */
    private function rowInt(array $row, string $key): int
    {
        $value = $row[$key] ?? null;

        if (! is_int($value) && ! is_string($value)) {
            self::fail("MariaDB row column {$key} was not integer-compatible.");
        }

        return (int) $value;
    }

    /**
     * @param  array<string, mixed>  $row
     */
    private function rowString(array $row, string $key): string
    {
        $value = $row[$key] ?? null;

        if (! is_string($value)) {
            self::fail("MariaDB row column {$key} was not a string.");
        }

        return $value;
    }

    /**
     * @return array<string, mixed>
     */
    private function rootRow(string $query): array
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $statement = $this->root->query($query);

        if ($statement === false) {
            self::fail('MariaDB integration query failed.');
        }

        $row = $statement->fetch(PDO::FETCH_ASSOC);

        if (! is_array($row)) {
            self::fail('MariaDB integration row was unavailable.');
        }

        $normalized = [];

        foreach ($row as $key => $value) {
            // @phpstan-ignore function.alreadyNarrowedType (PDO may return integer keys for numeric column aliases)
            if (! is_string($key)) {
                self::fail('MariaDB integration row contained a non-string key.');
            }

            $normalized[$key] = $value;
        }

        return $normalized;
    }

    private function rootValue(string $query): mixed
    {
        if (! $this->root instanceof PDO) {
            self::fail('MariaDB root connection is unavailable.');
        }

        $statement = $this->root->query($query);

        if ($statement === false) {
            self::fail('MariaDB integration query failed.');
        }

        return $statement->fetchColumn();
    }
}
