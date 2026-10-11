<?php

namespace Tests\Feature\Accounts;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Exceptions\CanaryAccountProvisioningConflict;
use App\Accounts\Models\IdentityCanaryAccount;
use App\CanaryIntegration\CanaryAccountProvisioner;
use App\CanaryIntegration\CanaryProvisioningDatabasePrivilegeVerifier;
use App\Identity\Models\Identity;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use PDO;
use Tests\TestCase;

final class CanaryProvisioningMariaDbIntegrationTest extends TestCase
{
    use RefreshDatabase;

    private const DATABASE = 'oteryn_canary_provisioning_test';

    private const USER = 'oteryn_provisioning_test';

    private const PASSWORD = 'oteryn-provisioning-test-password';

    private ?PDO $root = null;

    protected function setUp(): void
    {
        parent::setUp();

        $host = getenv('CANARY_PROVISIONING_INTEGRATION_HOST');

        if (! is_string($host) || $host === '') {
            $this->markTestSkipped('MariaDB provisioning integration environment is not configured.');
        }

        $port = getenv('CANARY_PROVISIONING_INTEGRATION_PORT');
        $rootPassword = getenv('CANARY_PROVISIONING_INTEGRATION_ROOT_PASSWORD');
        $port = is_string($port) && $port !== '' ? $port : '3306';
        $rootPassword = is_string($rootPassword) ? $rootPassword : '';

        $this->root = new PDO(
            "mysql:host={$host};port={$port};charset=utf8mb4",
            'root',
            $rootPassword,
            [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION],
        );

        $this->resetDatabase();
        $this->configureProvisioningConnection($host, $port);
    }

    protected function tearDown(): void
    {
        DB::purge(CanaryAccountProvisioner::CONNECTION);

        if ($this->root instanceof PDO) {
            $this->root->exec('DROP DATABASE IF EXISTS `'.self::DATABASE.'`');
            $this->root->exec("DROP USER IF EXISTS '".self::USER."'@'%'");
        }

        parent::tearDown();
    }

    public function test_real_mariadb_grants_trigger_and_forward_recovery_match_contract(): void
    {
        $violations = (new CanaryProvisioningDatabasePrivilegeVerifier)->inspect();
        self::assertSame([], $violations);

        $provisioner = new CanaryAccountProvisioner;
        $name = 'op'.str_repeat('f', 30);
        $creationEpoch = 1_800_000_006;

        $firstId = $provisioner->provision($name, $creationEpoch);
        $recoveredId = $provisioner->provision($name, $creationEpoch);

        self::assertSame($firstId, $recoveredId);
        self::assertSame(1, $this->rootCount('SELECT COUNT(*) FROM `'.self::DATABASE."`.`accounts` WHERE `name` = '{$name}'"));
        self::assertSame(3, $this->rootCount('SELECT COUNT(*) FROM `'.self::DATABASE."`.`account_vipgroups` WHERE `account_id` = {$firstId}"));

        $storedPassword = $this->rootValue('SELECT `password` FROM `'.self::DATABASE."`.`accounts` WHERE `id` = {$firstId}");
        self::assertIsString($storedPassword);
        self::assertMatchesRegularExpression('/^[0-9a-f]{40}$/', $storedPassword);

        try {
            DB::connection(CanaryAccountProvisioner::CONNECTION)
                ->table('accounts')
                ->select('password')
                ->where('id', $firstId)
                ->first();
            self::fail('Provisioning credential must not be able to read accounts.password.');
        } catch (QueryException) {
            // Expected database-enforced denial.
        }

        $this->expectException(CanaryAccountProvisioningConflict::class);
        $provisioner->provision($name, $creationEpoch + 1);
    }

    public function test_platform_saga_finalizes_a_previously_committed_canary_account_without_creating_a_duplicate(): void
    {
        $identity = Identity::query()->create([
            'email' => 'forward-recovery@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $name = 'op'.str_repeat('a', 30);
        $creationEpoch = 1_800_000_007;
        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'provisioning_name' => $name,
            'canary_creation_epoch' => $creationEpoch,
            'status' => IdentityCanaryAccount::STATUS_PENDING,
        ]);

        // Equivalent durable state to: Canary commit succeeded, but the Platform
        // finalization transaction did not commit.
        $committedAccountId = (new CanaryAccountProvisioner)->provision($name, $creationEpoch);

        $binding = $this->app->make(ProvisionCanaryAccount::class)->execute($identity->id);

        self::assertTrue($binding->isReady());
        self::assertSame($committedAccountId, $binding->canary_account_id);
        self::assertSame(1, $this->rootCount('SELECT COUNT(*) FROM `'.self::DATABASE."`.`accounts` WHERE `name` = '{$name}'"));
        self::assertSame(3, $this->rootCount('SELECT COUNT(*) FROM `'.self::DATABASE."`.`account_vipgroups` WHERE `account_id` = {$committedAccountId}"));
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
            .'`name` varchar(32) NOT NULL,'
            .'`password` varchar(255) NOT NULL,'
            .'`email` varchar(255) NOT NULL DEFAULT \'\','
            .'`creation` int(11) UNSIGNED NOT NULL DEFAULT 0,'
            .'PRIMARY KEY (`id`), UNIQUE KEY `accounts_unique` (`name`)'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );
        $this->root->exec(
            'CREATE TABLE `'.self::DATABASE.'`.`account_vipgroups` ('
            .'`id` int(11) UNSIGNED NOT NULL AUTO_INCREMENT,'
            .'`account_id` int(11) UNSIGNED NOT NULL,'
            .'`name` varchar(128) NOT NULL,'
            .'`customizable` boolean NOT NULL DEFAULT 1,'
            .'PRIMARY KEY (`id`, `account_id`),'
            .'CONSTRAINT `account_vipgroups_accounts_fk` FOREIGN KEY (`account_id`) REFERENCES `accounts` (`id`) ON DELETE CASCADE'
            .') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4',
        );
        $this->root->exec(
            'CREATE TRIGGER `'.self::DATABASE.'`.`oncreate_accounts` AFTER INSERT ON `'.self::DATABASE.'`.`accounts` '
            .'FOR EACH ROW BEGIN '
            .'INSERT INTO `'.self::DATABASE.'`.`account_vipgroups` (`account_id`, `name`, `customizable`) VALUES (NEW.`id`, \'Enemies\', 0); '
            .'INSERT INTO `'.self::DATABASE.'`.`account_vipgroups` (`account_id`, `name`, `customizable`) VALUES (NEW.`id`, \'Friends\', 0); '
            .'INSERT INTO `'.self::DATABASE.'`.`account_vipgroups` (`account_id`, `name`, `customizable`) VALUES (NEW.`id`, \'Trading Partner\', 0); '
            .'END',
        );

        $this->root->exec(
            'GRANT INSERT (`name`, `password`, `email`, `creation`) ON `'.self::DATABASE.'`.`accounts` '
            ."TO '".self::USER."'@'%'",
        );
        $this->root->exec(
            'GRANT SELECT (`id`, `name`, `creation`) ON `'.self::DATABASE.'`.`accounts` '
            ."TO '".self::USER."'@'%'",
        );
    }

    private function configureProvisioningConnection(string $host, string $port): void
    {
        config()->set('database.connections.'.CanaryAccountProvisioner::CONNECTION, [
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

        DB::purge(CanaryAccountProvisioner::CONNECTION);
    }

    private function rootCount(string $query): int
    {
        $value = $this->rootValue($query);

        if (! is_int($value) && ! is_string($value)) {
            self::fail('MariaDB integration count query returned a non-scalar value.');
        }

        return (int) $value;
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
