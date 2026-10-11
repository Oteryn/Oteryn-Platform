<?php

namespace Tests\Feature\GameAuth\NativeTopology;

use App\GameAuth\Worlds\NativePreproductionStoreRefused;
use App\GameAuth\Worlds\NativeRouteRecords;
use Illuminate\Database\SQLiteConnection;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Tests\TestCase;

/**
 * ADR 0044 (ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 amendment, owner option a, 2026-10-11): the preproduction-store
 * mode of game-auth:native-topology:issue and game-auth:native-route:publish works only when every condition
 * holds, each violated condition refuses with its named reason and nothing written, and the default
 * disposable-store path is unchanged while the switch is off.
 *
 * The allowlisted store is a double: a connection whose configuration is the deployed MariaDB store
 * (driver mysql, host mariadb, port 3306, database oteryn_platform) and whose statements run on the
 * migrated in-memory test database, so the fence sees the deployed profile and the writes are observable.
 */
final class PreproductionStoreRouteCommandsTest extends TestCase
{
    use DatabaseMigrations;

    private const WORLD = 1;

    private const CHANNEL = 'ch1';

    private const PASSWORD = 'placeholder-not-a-secret';

    private const STORE = [
        'driver' => 'mysql',
        'host' => 'mariadb',
        'port' => '3306',
        'database' => 'oteryn_platform',
        'username' => 'oteryn_platform',
        'password' => self::PASSWORD,
        'unix_socket' => '',
        'prefix' => '',
    ];

    protected function tearDown(): void
    {
        if ($this->app !== null) {
            $this->app->detectEnvironment(fn (): string => 'testing');
            config(['game-auth.native_route_preproduction_store.enabled' => false]);
            DB::setDefaultConnection('sqlite');
            DB::purge('preprod_store');
            Artisan::call('migrate:fresh', ['--force' => true]);
        }
        parent::tearDown();
    }

    public function test_preproduction_store_mode_issues_and_publishes_exactly_one_route_record_with_every_condition_met(): void
    {
        $this->provisionedWorld();
        $this->preproductionStore(Str::uuid7()->toString());

        // First deploy: the Registry mints the WorldId; the deploy then stops on the mismatch (lib.sh).
        $first = $this->issue();
        self::assertSame(0, $first['exit']);
        self::assertSame('issued', $first['json']['result']);
        self::assertSame('preproduction-store-native-topology', $first['json']['purpose']);
        $worldId = self::text($first['json']['world_id']);
        self::assertSame(1, $this->publish()['exit']);

        // The owner copies the issued WorldId into OTERYN_STAGING_NATIVE_WORLD_ID and redeploys.
        config(['game-auth.native_admission.unverified_character_world_id' => $worldId]);
        $again = $this->issue();
        self::assertSame(0, $again['exit']);
        self::assertSame('retained', $again['json']['result']);
        self::assertSame($first['json']['channel_id'], $again['json']['channel_id']);

        $published = $this->publish();
        self::assertSame(0, $published['exit'], $published['output']);
        $route = $published['json'];
        self::assertSame($worldId, $route['world_id']);
        self::assertSame($first['json']['channel_id'], $route['channel_id']);
        self::assertSame(1, $route['route_version']);
        self::assertTrue($route['native_login_enabled']);
        self::assertSame('preproduction', $route['store']);
        self::assertSame('published', $route['result']);
        $record = (new NativeRouteRecords)->find($worldId, self::text($route['channel_id']));
        self::assertNotNull($record);
        self::assertSame($route['route_revision'], $record->routeRevision);
        self::assertSame(1, DB::table('game_channels')->whereNotNull('native_route_revision')->count());

        // Idempotent: the same values keep the revision and write no route column.
        $stamp = DB::table('game_channels')->sole()->updated_at;
        $repeat = $this->publish();
        self::assertSame(0, $repeat['exit']);
        self::assertSame('unchanged', $repeat['json']['result']);
        self::assertSame($route['route_revision'], $repeat['json']['route_revision']);
        self::assertSame($stamp, DB::table('game_channels')->sole()->updated_at);

        // An endpoint change advances the version; login disabled keeps the endpoint.
        $moved = $this->publish(['--port' => '7443']);
        self::assertSame(2, $moved['json']['route_version']);
        self::assertNotSame($route['route_revision'], $moved['json']['route_revision']);
        $disabled = $this->publish(['--port' => '7443', '--login-enabled' => 'false']);
        self::assertSame($moved['json']['route_revision'], $disabled['json']['route_revision']);
        self::assertFalse($disabled['json']['native_login_enabled']);
        self::assertSame('published', $disabled['json']['result']);

        // Audit: one event per successful run, actor-less, no credential or host in it.
        $actions = DB::table('admin_audit_events')->orderBy('id')->pluck('action')->all();
        self::assertSame([
            'game_auth.native_topology.preproduction_store_issue',
            'game_auth.native_topology.preproduction_store_issue',
            'game_auth.native_route.preproduction_store_publish',
            'game_auth.native_route.preproduction_store_publish',
            'game_auth.native_route.preproduction_store_publish',
            'game_auth.native_route.preproduction_store_publish',
        ], $actions);
        $audit = DB::table('admin_audit_events')->orderByDesc('id')->first();
        self::assertNotNull($audit);
        self::assertNull($audit->actor_identity_id);
        self::assertSame($route['channel_id'], $audit->target_id);
        self::assertStringNotContainsString(self::PASSWORD, self::text($audit->metadata));
        foreach ([$first, $again, $published, $repeat, $moved, $disabled] as $run) {
            self::assertStringNotContainsString(self::PASSWORD, $run['output']);
            self::assertStringNotContainsString('mariadb', $run['output']);
        }
    }

    public function test_default_path_is_unchanged_and_refuses_the_deployed_store_while_the_switch_is_off(): void
    {
        $this->provisionedWorld();
        foreach ([null, false, 'false'] as $switch) {
            $this->preproductionStore(Str::uuid7()->toString());
            config(['game-auth.native_route_preproduction_store.enabled' => $switch]);

            $issue = $this->issue();
            self::assertSame(1, $issue['exit']);
            self::assertStringContainsString('Disposable native topology issuance/readback failed.', $issue['output']);
            $publish = $this->publish();
            self::assertSame(1, $publish['exit']);
            self::assertStringContainsString('Disposable native route publication failed.', $publish['output']);
            self::assertStringNotContainsString('NATIVE_PREPROD_STORE', $issue['output'].$publish['output']);
            $this->assertNothingWritten();
        }
    }

    public function test_each_violated_fence_condition_refuses_with_its_named_reason_and_nothing_written(): void
    {
        $this->provisionedWorld();
        $world = Str::uuid7()->toString();
        $cases = [
            'malformed switch' => [NativePreproductionStoreRefused::SWITCH_INVALID, fn () => config(['game-auth.native_route_preproduction_store.enabled' => 'yes'])],
            'testing environment' => [NativePreproductionStoreRefused::ENVIRONMENT, fn () => $this->useEnvironment('testing')],
            'local environment' => [NativePreproductionStoreRefused::ENVIRONMENT, fn () => $this->useEnvironment('local')],
            'staging environment' => [NativePreproductionStoreRefused::ENVIRONMENT, fn () => $this->useEnvironment('staging')],
            'production environment' => [NativePreproductionStoreRefused::ENVIRONMENT, fn () => $this->useEnvironment('production')],
            'world unconfigured' => [NativePreproductionStoreRefused::WORLD_UNCONFIGURED, fn () => config(['game-auth.native_admission.unverified_character_world_id' => null])],
            'world not canonical' => [NativePreproductionStoreRefused::WORLD_UNCONFIGURED, fn () => config(['game-auth.native_admission.unverified_character_world_id' => strtoupper($world)])],
            'world not UUIDv7' => [NativePreproductionStoreRefused::WORLD_UNCONFIGURED, fn () => config(['game-auth.native_admission.unverified_character_world_id' => Str::uuid()->toString()])],
            'other host' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['host' => 'db'])],
            'loopback host' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['host' => '127.0.0.1'])],
            'other database' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['database' => 'oteryn'])],
            'test database' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['database' => 'oteryn_concurrency'])],
            'other port' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['port' => '3307'])],
            'unix socket' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['unix_socket' => '/run/mysqld/mysqld.sock'])],
            'other driver' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['driver' => 'mariadb'])],
            'read replica' => [NativePreproductionStoreRefused::DATABASE, fn () => $this->preproductionStore($world, ['read' => ['host' => ['replica']]])],
            'disposable sqlite store' => [NativePreproductionStoreRefused::DATABASE, fn () => DB::setDefaultConnection('sqlite')],
        ];

        foreach ($cases as $case => [$reason, $violate]) {
            $this->preproductionStore($world);
            $violate();

            foreach ([$this->issue(), $this->publish()] as $run) {
                self::assertSame(1, $run['exit'], $case);
                self::assertStringContainsString($reason, $run['output'], $case);
                self::assertStringNotContainsString(self::PASSWORD, $run['output'], $case);
            }
            $this->useEnvironment('testing');
            $this->assertNothingWritten();
        }

        // An outer transaction is refused before any write.
        $this->preproductionStore($world);
        DB::transaction(function (): void {
            self::assertStringContainsString(NativePreproductionStoreRefused::TRANSACTION_OPEN, $this->issue()['output']);
            self::assertStringContainsString(NativePreproductionStoreRefused::TRANSACTION_OPEN, $this->publish()['output']);
        });
        $this->assertNothingWritten();
    }

    public function test_world_and_route_conditions_refuse_with_their_named_reasons_and_nothing_written(): void
    {
        $this->provisionedWorld();
        $configured = Str::uuid7()->toString();
        $this->preproductionStore($configured);

        // No such World row.
        foreach ([$this->issue(['--world-row-id' => '99']), $this->publish(['--world-row-id' => '99'])] as $run) {
            self::assertSame(1, $run['exit']);
            self::assertStringContainsString(NativePreproductionStoreRefused::WORLD_ROW_MISSING, $run['output']);
        }
        $this->assertNothingWritten();

        // The World row has no WorldId yet, or a WorldId other than the configured native World.
        self::assertStringContainsString(NativePreproductionStoreRefused::WORLD_MISMATCH, $this->publish()['output']);
        $other = Str::uuid7()->toString();
        DB::table('game_worlds')->where('id', self::WORLD)->update(['world_id' => $other]);
        $issue = $this->issue();
        self::assertSame(1, $issue['exit']);
        self::assertStringContainsString(NativePreproductionStoreRefused::WORLD_MISMATCH, $issue['output']);
        self::assertStringContainsString($other, $issue['output']);
        self::assertStringContainsString(NativePreproductionStoreRefused::WORLD_MISMATCH, $this->publish()['output']);
        self::assertSame(0, DB::table('game_channels')->count());

        // The configured World without an issued Channel.
        DB::table('game_worlds')->where('id', self::WORLD)->update(['world_id' => $configured]);
        self::assertStringContainsString(NativePreproductionStoreRefused::CHANNEL_NOT_ISSUED, $this->publish()['output']);

        // Another World row on the store holds a WorldId.
        $this->provisionedWorld(2, 'second-world');
        DB::table('game_worlds')->where('id', 2)->update(['world_id' => Str::uuid7()->toString()]);
        self::assertStringContainsString(NativePreproductionStoreRefused::FOREIGN_WORLD, $this->issue()['output']);
        self::assertSame(0, DB::table('game_channels')->count());
        DB::table('game_worlds')->where('id', 2)->update(['world_id' => null]);

        // Another Channel on the store already holds a route record.
        self::assertSame(0, $this->issue(['--channel-key' => 'ch0'])['exit']);
        self::assertSame(0, $this->publish(['--channel-key' => 'ch0'])['exit']);
        self::assertSame(0, $this->issue()['exit']);
        DB::table('admin_audit_events')->delete();
        $foreign = $this->publish();
        self::assertSame(1, $foreign['exit']);
        self::assertStringContainsString(NativePreproductionStoreRefused::FOREIGN_ROUTE, $foreign['output']);
        self::assertNull(DB::table('game_channels')->where('channel_key', self::CHANNEL)->sole()->native_route_revision);
        self::assertSame(0, DB::table('admin_audit_events')->count());
    }

    /** @param array<string, mixed> $override */
    private function preproductionStore(string $worldId, array $override = []): void
    {
        $config = array_merge(self::STORE, $override);
        config([
            'database.connections.preprod_store' => $config,
            'game-auth.native_route_preproduction_store.enabled' => true,
            'game-auth.native_admission.unverified_character_world_id' => $worldId,
        ]);
        $pdo = DB::connection('sqlite')->getPdo();
        DB::extend('preprod_store', fn (array $config, string $name): SQLiteConnection => new SQLiteConnection(
            $pdo,
            is_string($config['database'] ?? null) ? $config['database'] : '',
            '',
            array_merge($config, ['name' => $name]),
        ));
        DB::purge('preprod_store');
        DB::setDefaultConnection('preprod_store');
        $this->useEnvironment('preproduction');
    }

    private function provisionedWorld(int $id = self::WORLD, string $slug = 'preprod-world'): void
    {
        self::assertSame(0, Artisan::call('game-auth:world:ensure', [
            '--id' => (string) $id,
            '--slug' => $slug,
            '--name' => 'Preproduction world',
            '--region' => 'LAN',
            '--host' => '127.0.0.1',
            '--port' => '7172',
            '--status' => 'online',
            '--login-enabled' => '1',
        ]));
    }

    /**
     * @param  array<string, string>  $override
     * @return array{exit: int, output: string, json: array<string, mixed>}
     */
    private function issue(array $override = []): array
    {
        return $this->runCommand('game-auth:native-topology:issue', array_merge([
            '--world-row-id' => (string) self::WORLD, '--channel-key' => self::CHANNEL,
        ], $override));
    }

    /**
     * @param  array<string, string>  $override
     * @return array{exit: int, output: string, json: array<string, mixed>}
     */
    private function publish(array $override = []): array
    {
        return $this->runCommand('game-auth:native-route:publish', array_merge([
            '--world-row-id' => (string) self::WORLD,
            '--channel-key' => self::CHANNEL,
            '--host' => '192.168.1.20',
            '--port' => '7172',
            '--tls-server-name' => 'node-1.preprod.oteryn.internal',
            '--login-enabled' => 'true',
        ], $override));
    }

    /**
     * @param  array<string, string>  $options
     * @return array{exit: int, output: string, json: array<string, mixed>}
     */
    private function runCommand(string $command, array $options): array
    {
        $exit = Artisan::call($command, $options);
        $output = Artisan::output();

        $json = [];
        if ($exit === 0) {
            $decoded = json_decode($output, true, 512, JSON_THROW_ON_ERROR);
            self::assertIsArray($decoded);
            foreach ($decoded as $key => $value) {
                $json[(string) $key] = $value;
            }
        }

        return ['exit' => $exit, 'output' => $output, 'json' => $json];
    }

    private static function text(mixed $value): string
    {
        if (! is_string($value)) {
            self::fail('Expected a string value.');
        }

        return $value;
    }

    private function assertNothingWritten(): void
    {
        self::assertSame(0, DB::connection('sqlite')->table('game_worlds')->whereNotNull('world_id')->count());
        self::assertSame(0, DB::connection('sqlite')->table('game_channels')->count());
        self::assertSame(0, DB::connection('sqlite')->table('admin_audit_events')->count());
    }

    private function useEnvironment(string $environment): void
    {
        $this->app->detectEnvironment(fn (): string => $environment);
    }
}
