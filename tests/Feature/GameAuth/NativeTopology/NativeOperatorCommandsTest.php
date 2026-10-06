<?php

namespace Tests\Feature\GameAuth\NativeTopology;

use App\GameAuth\Worlds\NativeRouteRecords;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

/** ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2: the two operator commands and the shared disposable-store guard. */
final class NativeOperatorCommandsTest extends TestCase
{
    use DatabaseMigrations;

    private const WORLD = 17;

    private const CHANNEL = 'entry-room';

    private const KEY_ID = 'preprod-fresh-1';

    /** @var list<string> */
    private array $cleanup = [];

    private mixed $originalDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        $this->originalDatabase = config('database.connections.sqlite.database');
        config(['game-auth.native_evidence.fresh_key_purpose' => 'fresh_admission']);
    }

    protected function tearDown(): void
    {
        if ($this->app !== null) {
            $this->app->detectEnvironment(fn (): string => 'testing');
            config([
                'database.default' => 'sqlite',
                'database.connections.sqlite.database' => $this->originalDatabase,
            ]);
            DB::purge();
            Artisan::call('migrate:fresh', ['--force' => true]);
        }
        foreach (array_reverse($this->cleanup) as $path) {
            $this->remove($path);
        }
        parent::tearDown();
    }

    public function test_happy_path_in_preproduction_issues_publishes_and_prints_readback_receipts(): void
    {
        $run = $this->retainedRun();
        $this->provisionedWorld();
        $this->environment('preproduction');

        self::assertSame(0, Artisan::call('game-auth:native-topology:issue', [
            '--world-row-id' => (string) self::WORLD, '--channel-key' => self::CHANNEL,
        ]));
        $issued = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($issued);

        $route = $this->publishRoute();
        self::assertSame($issued['world_id'], $route['world_id']);
        self::assertSame($issued['channel_id'], $route['channel_id']);
        self::assertSame(1, $route['route_version']);
        self::assertTrue($route['native_login_enabled']);
        $record = (new NativeRouteRecords)->find($route['world_id'], $route['channel_id']);
        self::assertNotNull($record);
        self::assertSame($record->routeRevision, $route['route_revision']);
        self::assertSame('node.preprod.test', $record->host);

        $this->highWater($run.'/witness');
        $publicKey = $this->publicKeyFile($run);
        self::assertSame(0, Artisan::call('game-auth:native-trust:publish-key', [
            '--key-id' => self::KEY_ID, '--public-key-file' => $publicKey,
        ]));
        self::assertSame(['key_id' => self::KEY_ID, 'profile_version' => 1], json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR));
        self::assertSame(1, DB::table('native_game_signing_trust_profiles')->count());
        $profile = DB::table('native_game_signing_trust_profiles')->sole();
        self::assertSame('urn:oteryn:platform:game-admission', $profile->issuer);
        self::assertSame('oteryn-pre-admission-v1', $profile->profile);
        self::assertSame('fresh_admission', $profile->key_purpose);
        self::assertSame(self::KEY_ID, DB::table('native_game_signing_trust_key_versions')->sole()->key_id);
        self::assertNotSame([], $this->files($run.'/witness'));
    }

    public function test_endpoint_change_advances_route_version_and_login_disabled_keeps_the_endpoint(): void
    {
        $this->retainedRun();
        $this->provisionedWorld();
        $this->environment('preproduction');
        $this->issue();

        $first = $this->publishRoute();
        self::assertSame($first, $this->publishRoute());

        $moved = $this->publishRoute(['--port' => '7443']);
        self::assertSame(2, $moved['route_version']);
        self::assertNotSame($first['route_revision'], $moved['route_revision']);

        $disabled = $this->publishRoute(['--port' => '7443', '--login-enabled' => 'false']);
        self::assertSame($moved['route_revision'], $disabled['route_revision']);
        self::assertSame(2, $disabled['route_version']);
        self::assertFalse($disabled['native_login_enabled']);
        $channel = DB::table('game_channels')->sole();
        self::assertSame('node.preprod.test', $channel->native_route_host);
        self::assertEquals(7443, $channel->native_route_port);
        self::assertFalse((bool) $channel->native_login_enabled);
    }

    public function test_each_command_refuses_outside_testing_and_preproduction_with_nothing_written(): void
    {
        $run = $this->retainedRun();
        $this->provisionedWorld();
        $this->highWater($run.'/witness');
        $publicKey = $this->publicKeyFile($run);

        foreach (['local', 'staging', 'production'] as $environment) {
            $this->environment($environment);
            $this->assertCommandsRefused($publicKey);
            $this->environment('testing');
            $this->assertNothingWritten($run.'/witness');
        }
    }

    public function test_each_command_refuses_a_non_disposable_preproduction_store_with_nothing_written(): void
    {
        // A MariaDB store as in the staging deployment: refused before any connection is opened.
        config(['database.connections.staging_mariadb' => [
            'driver' => 'mysql', 'host' => 'mariadb', 'port' => '3306', 'database' => 'oteryn',
            'username' => 'oteryn', 'password' => 'placeholder', 'unix_socket' => '',
        ], 'database.default' => 'staging_mariadb']);
        DB::purge();
        $outsideWitness = $this->temporaryDirectory('oteryn-native-witness-');
        $this->highWater($outsideWitness);
        $publicKey = $this->publicKeyFile($outsideWitness.'-key');
        $this->environment('preproduction');
        $this->assertCommandsRefused($publicKey, true);
        self::assertSame([], $this->files($outsideWitness));
        $this->environment('testing');
        config(['database.default' => 'sqlite']);
        DB::purge();

        // A SQLite file outside a per-run directory, then a symlinked file in a per-run directory.
        $outside = $this->temporaryDirectory('oteryn-persistent-');
        $this->migratedStore($outside.'/oteryn-native-topology.sqlite');
        $this->highWater($outside.'/witness');
        $this->environment('preproduction');
        $this->assertCommandsRefused($publicKey, true);
        $this->environment('testing');
        $this->assertNothingWritten($outside.'/witness', true);

        $run = $this->temporaryDirectory('oteryn-native-topology-');
        self::assertTrue(symlink($outside.'/oteryn-native-topology.sqlite', $run.'/oteryn-native-topology.sqlite'));
        $this->useStore($run.'/oteryn-native-topology.sqlite');
        self::assertTrue(mkdir($run.'/witness', 0700));
        $this->highWater($run.'/witness');
        $this->environment('preproduction');
        $this->assertCommandsRefused($publicKey, true);
        $this->environment('testing');
        self::assertSame([], $this->files($run.'/witness'));
        $this->useStore($outside.'/oteryn-native-topology.sqlite');
        $this->assertNothingWritten($outside.'/witness', true);
    }

    public function test_symlinked_run_directory_to_a_persistent_directory_is_refused_in_testing_and_preproduction(): void
    {
        $persistent = $this->temporaryDirectory('oteryn-persistent-');
        $this->migratedStore($persistent.'/oteryn-native-topology.sqlite');
        $this->provisionedWorld();
        self::assertTrue(mkdir($persistent.'/witness', 0700));
        $publicKey = $this->publicKeyFile($persistent.'-key');

        $link = $this->temporaryPath('oteryn-native-topology-');
        self::assertTrue(symlink($persistent, $link));
        self::assertTrue(is_file($link.'/oteryn-native-topology.sqlite') && ! is_link($link.'/oteryn-native-topology.sqlite'));

        foreach (['testing', 'preproduction'] as $environment) {
            $this->useStore($link.'/oteryn-native-topology.sqlite');
            $this->highWater($link.'/witness');
            $this->environment($environment);
            $this->assertCommandsRefused($publicKey);
            $this->environment('testing');
            $this->useStore($persistent.'/oteryn-native-topology.sqlite');
            $this->assertNothingWritten($persistent.'/witness');
        }
    }

    public function test_issue_command_refuses_outside_preproduction_profiles_without_issuance(): void
    {
        $run = $this->retainedRun();
        $this->provisionedWorld();
        foreach (['local', 'staging', 'production'] as $environment) {
            $this->environment($environment);
            self::assertSame(1, $this->issueExitCode());
        }
        $this->environment('testing');
        $this->assertNoIssuance();

        config(['database.connections.sqlite.database' => ':memory:']);
        DB::purge();
        $this->environment('preproduction');
        self::assertSame(1, $this->issueExitCode());
        $this->loopbackConcurrencyStore();
        self::assertSame(1, $this->issueExitCode());
        $this->environment('testing');
        $this->useStore($run.'/oteryn-native-topology.sqlite');
        $this->assertNoIssuance();
    }

    public function test_new_commands_require_the_retained_run_file_even_in_testing(): void
    {
        // In testing the shared guard admits `:memory:` (this test's default store) and the loopback
        // concurrency database, but neither names a per-run directory.
        $this->provisionedWorld();
        DB::table('game_worlds')->where('id', self::WORLD)->update(['world_id' => '01890f4e-7c00-7000-8000-000000000001']);
        DB::table('game_channels')->insert([
            'game_world_id' => self::WORLD, 'channel_id' => '01890f4e-7c00-7000-8000-000000000002',
            'channel_key' => self::CHANNEL, 'created_at' => now(), 'updated_at' => now(),
        ]);
        $otherRun = $this->temporaryDirectory('oteryn-native-topology-');
        self::assertTrue(mkdir($otherRun.'/witness', 0700));
        $this->highWater($otherRun.'/witness');
        $publicKey = $this->publicKeyFile($otherRun);

        self::assertSame(1, Artisan::call('game-auth:native-route:publish', $this->routeOptions()));
        self::assertSame(1, $this->publishKeyExitCode($publicKey));
        $this->assertNothingWritten($otherRun.'/witness', true);

        $this->loopbackConcurrencyStore();
        self::assertSame(1, Artisan::call('game-auth:native-route:publish', $this->routeOptions()));
        self::assertSame(1, $this->publishKeyExitCode($publicKey));
        self::assertSame([], $this->files($otherRun.'/witness'));
        config(['database.default' => 'sqlite']);
        DB::table('game_channels')->delete();
        DB::table('game_worlds')->update(['world_id' => null]);
    }

    public function test_trust_command_refuses_a_high_water_directory_outside_the_current_run(): void
    {
        $run = $this->retainedRun();
        $this->environment('preproduction');
        $publicKey = $this->publicKeyFile($run);

        $outside = $this->temporaryDirectory('oteryn-native-witness-');
        $otherRun = $this->temporaryDirectory('oteryn-native-topology-');
        self::assertTrue(mkdir($otherRun.'/witness', 0700));
        $linkedWitness = $run.'/linked-witness';
        self::assertTrue(symlink($outside, $linkedWitness));
        $linkedParent = $this->temporaryPath('oteryn-native-topology-');
        self::assertTrue(symlink($run, $linkedParent));
        self::assertTrue(mkdir($run.'/witness', 0700));

        foreach ([
            $outside,
            $otherRun.'/witness',
            $linkedWitness,
            $linkedParent.'/witness',
            $run.'/witness/../witness',
            $run,
            $run.'/missing',
        ] as $directory) {
            $this->highWater($directory);
            self::assertSame(1, $this->publishKeyExitCode($publicKey), $directory);
        }
        $this->assertNothingWritten($run.'/witness', true);
        self::assertSame([], $this->files($outside));
        self::assertSame([], $this->files($otherRun.'/witness'));
    }

    public function test_malformed_selector_port_and_key_file_are_refused_before_any_write(): void
    {
        $run = $this->retainedRun();
        $this->provisionedWorld();
        $this->environment('preproduction');
        $this->issue();
        $this->highWater($run.'/witness');

        foreach ([
            ['--world-row-id' => '0'], ['--world-row-id' => '17 '], ['--world-row-id' => 'abc'], ['--world-row-id' => '18'],
            ['--channel-key' => 'Entry-Room'], ['--channel-key' => "entry-room\n"], ['--channel-key' => 'other-room'],
            ['--port' => '0'], ['--port' => '65536'], ['--port' => '07172'], ['--port' => '7172 '], ['--port' => '-1'],
            ['--host' => 'https://node.preprod.test'], ['--host' => ''],
            ['--tls-server-name' => '127.0.0.1'], ['--tls-server-name' => ''],
            ['--login-enabled' => '1'], ['--login-enabled' => 'TRUE'], ['--login-enabled' => null],
        ] as $override) {
            self::assertSame(1, Artisan::call('game-auth:native-route:publish', $this->routeOptions($override)), json_encode($override, JSON_THROW_ON_ERROR));
            self::assertStringNotContainsString('Exception', Artisan::output());
        }
        $this->assertNoRoute();

        $short = $run.'/short.key';
        file_put_contents($short, random_bytes(31));
        $long = $run.'/long.key';
        file_put_contents($long, random_bytes(33));
        $valid = $this->publicKeyFile($run);
        $linked = $run.'/linked.key';
        self::assertTrue(symlink($valid, $linked));
        foreach ([$short, $long, $linked, $run, $run.'/missing.key', ''] as $file) {
            self::assertSame(1, $this->publishKeyExitCode($file), $file);
        }
        foreach (['', 'bad key', str_repeat('k', 65)] as $keyId) {
            self::assertSame(1, Artisan::call('game-auth:native-trust:publish-key', [
                '--key-id' => $keyId, '--public-key-file' => $valid,
            ]));
        }
        $this->assertNothingWritten($run.'/witness', true);
    }

    private function assertCommandsRefused(string $publicKey, bool $withoutWorld = false): void
    {
        if (! $withoutWorld) {
            self::assertSame(1, $this->issueExitCode());
        }
        self::assertSame(1, Artisan::call('game-auth:native-route:publish', $this->routeOptions()));
        self::assertStringContainsString('Disposable native route publication failed.', Artisan::output());
        self::assertSame(1, $this->publishKeyExitCode($publicKey));
        self::assertStringContainsString('Disposable native trusted key publication failed.', Artisan::output());
    }

    private function assertNothingWritten(string $witness, bool $issued = false): void
    {
        if (! $issued) {
            $this->assertNoIssuance();
        }
        $this->assertNoRoute();
        self::assertSame(0, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame(0, DB::table('native_game_signing_trust_key_versions')->count());
        self::assertSame([], $this->files($witness));
    }

    private function assertNoIssuance(): void
    {
        self::assertNull(DB::table('game_worlds')->where('id', self::WORLD)->value('world_id'));
        self::assertSame(0, DB::table('game_channels')->count());
    }

    private function assertNoRoute(): void
    {
        self::assertSame(0, DB::table('game_channels')->whereNotNull('native_route_host')->count());
        self::assertSame(0, DB::table('game_channels')->where('native_login_enabled', true)->count());
    }

    /**
     * @param  array<string, string|null>  $override
     * @return array<string, string>
     */
    private function routeOptions(array $override = []): array
    {
        return array_filter(array_merge([
            '--world-row-id' => (string) self::WORLD,
            '--channel-key' => self::CHANNEL,
            '--host' => 'node.preprod.test',
            '--port' => '7172',
            '--tls-server-name' => 'node.preprod.test',
            '--login-enabled' => 'true',
        ], $override), fn (?string $value): bool => $value !== null);
    }

    /**
     * @param  array<string, string|null>  $override
     * @return array{world_id: string, channel_id: string, route_version: int, route_revision: string, native_login_enabled: bool}
     */
    private function publishRoute(array $override = []): array
    {
        self::assertSame(0, Artisan::call('game-auth:native-route:publish', $this->routeOptions($override)));
        $receipt = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);
        self::assertIsArray($receipt);
        self::assertSame(['world_id', 'channel_id', 'route_version', 'route_revision', 'native_login_enabled'], array_keys($receipt));
        self::assertIsString($receipt['world_id']);
        self::assertIsString($receipt['channel_id']);
        self::assertIsInt($receipt['route_version']);
        self::assertIsString($receipt['route_revision']);
        self::assertIsBool($receipt['native_login_enabled']);

        return [
            'world_id' => $receipt['world_id'],
            'channel_id' => $receipt['channel_id'],
            'route_version' => $receipt['route_version'],
            'route_revision' => $receipt['route_revision'],
            'native_login_enabled' => $receipt['native_login_enabled'],
        ];
    }

    private function publishKeyExitCode(string $publicKey): int
    {
        return Artisan::call('game-auth:native-trust:publish-key', [
            '--key-id' => self::KEY_ID, '--public-key-file' => $publicKey,
        ]);
    }

    private function issue(): void
    {
        self::assertSame(0, $this->issueExitCode());
    }

    private function issueExitCode(): int
    {
        return Artisan::call('game-auth:native-topology:issue', [
            '--world-row-id' => (string) self::WORLD, '--channel-key' => self::CHANNEL,
        ]);
    }

    private function provisionedWorld(): void
    {
        self::assertSame(0, Artisan::call('game-auth:world:ensure', [
            '--id' => (string) self::WORLD,
            '--slug' => 'disposable-entry',
            '--name' => 'Disposable entry',
            '--region' => 'TEST',
            '--host' => '127.0.0.1',
            '--port' => '7172',
            '--status' => 'maintenance',
            '--login-enabled' => '0',
        ]));
    }

    private function retainedRun(): string
    {
        $run = $this->temporaryDirectory('oteryn-native-topology-');
        $this->migratedStore($run.'/oteryn-native-topology.sqlite');

        return $run;
    }

    private function migratedStore(string $database): void
    {
        self::assertNotFalse(file_put_contents($database, ''));
        $this->useStore($database);
        self::assertSame(0, Artisan::call('migrate:fresh', ['--force' => true]));
        self::assertTrue(Schema::hasTable('native_game_evidence_witness_stores'));
    }

    private function useStore(string $database): void
    {
        config(['database.default' => 'sqlite', 'database.connections.sqlite.database' => $database]);
        DB::purge();
    }

    private function loopbackConcurrencyStore(): void
    {
        config(['database.connections.loopback_concurrency' => [
            'driver' => 'mysql', 'host' => '127.0.0.1', 'port' => '3306', 'database' => 'oteryn_concurrency',
            'username' => 'oteryn', 'password' => 'placeholder', 'unix_socket' => '',
        ], 'database.default' => 'loopback_concurrency']);
        DB::purge('loopback_concurrency');
    }

    private function highWater(string $directory): void
    {
        if (! is_dir($directory) && ! is_link($directory) && basename($directory) === 'witness') {
            self::assertTrue(mkdir($directory, 0700));
        }
        config(['game-auth.native_evidence.high_water_directory' => $directory]);
    }

    private function publicKeyFile(string $directory): string
    {
        if (! is_dir($directory)) {
            self::assertTrue(mkdir($directory, 0700));
            $this->cleanup[] = $directory;
        }
        $path = $directory.'/issuer.pub';
        self::assertNotFalse(file_put_contents($path, sodium_crypto_sign_publickey(sodium_crypto_sign_keypair())));

        return $path;
    }

    private function environment(string $environment): void
    {
        $this->app->detectEnvironment(fn (): string => $environment);
    }

    private function temporaryDirectory(string $prefix): string
    {
        $path = $this->temporaryPath($prefix);
        self::assertTrue(mkdir($path, 0700));

        return $path;
    }

    private function temporaryPath(string $prefix): string
    {
        $root = realpath(sys_get_temp_dir());
        self::assertIsString($root);
        $path = $root.'/'.$prefix.bin2hex(random_bytes(8));
        $this->cleanup[] = $path;

        return $path;
    }

    /** @return list<string> */
    private function files(string $directory): array
    {
        return is_dir($directory) ? array_values(array_diff(scandir($directory) ?: [], ['.', '..'])) : [];
    }

    private function remove(string $path): void
    {
        if (is_link($path) || is_file($path)) {
            unlink($path);

            return;
        }
        if (! is_dir($path)) {
            return;
        }
        foreach (array_diff(scandir($path) ?: [], ['.', '..']) as $entry) {
            $this->remove($path.'/'.$entry);
        }
        rmdir($path);
    }
}
