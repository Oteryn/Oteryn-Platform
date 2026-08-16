<?php

namespace Tests\Feature\CharacterProfiles\Concurrency;

use App\Accounts\Models\IdentityCanaryAccount;
use App\CharacterProfiles\CharacterProfilePreferenceService;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;
use Throwable;

final class CharacterProfilePreferenceConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    private ?string $canaryDatabasePath = null;

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('CHARACTER_PROFILE_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB character-profile concurrency workflow with pcntl.');
        }
    }

    protected function tearDown(): void
    {
        DB::purge('canary');

        if ($this->canaryDatabasePath !== null && is_file($this->canaryDatabasePath)) {
            unlink($this->canaryDatabasePath);
        }

        parent::tearDown();
    }

    public function test_concurrent_main_character_selection_preserves_single_main_invariant(): void
    {
        $this->configureCanaryDatabase();

        $identity = Identity::query()->create([
            'email' => uniqid('character-profile-concurrency-', true).'@example.com',
            'password' => Hash::make('Concurrency-Character-Profile-9!'),
        ]);
        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => 7001,
            'provisioning_name' => 'ready_'.$identity->id,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);
        $this->seedCharacters();
        DB::purge('canary');

        $identityId = (int) $identity->id;
        $results = $this->racePair(
            fn (): string => $this->selectMainCharacter($identityId, 'Alpha Knight'),
            fn (): string => $this->selectMainCharacter($identityId, 'Beta Druid'),
        );

        sort($results);
        self::assertSame(['success', 'success'], $results);

        DB::purge();
        DB::reconnect();

        $preferences = CharacterProfilePreference::query()
            ->where('identity_id', $identityId)
            ->orderBy('canary_player_id')
            ->get();
        self::assertCount(2, $preferences);

        $mainPreferences = $preferences->filter(
            fn (CharacterProfilePreference $preference): bool => $preference->is_main_character,
        );
        self::assertCount(1, $mainPreferences);

        $mainPreference = $mainPreferences->first();
        self::assertInstanceOf(CharacterProfilePreference::class, $mainPreference);
        self::assertContains($mainPreference->canary_player_id, [1, 2]);
    }

    private function selectMainCharacter(int $identityId, string $characterName): string
    {
        $identity = Identity::query()->findOrFail($identityId);

        $this->app->make(CharacterProfilePreferenceService::class)->update($identity, $characterName, [
            'public_comment' => null,
            'show_account_association' => false,
            'show_status' => false,
            'show_guild' => false,
            'show_house' => false,
            'show_skills' => false,
            'show_deaths' => false,
            'show_kills' => false,
            'is_main_character' => true,
        ]);

        return 'success';
    }

    private function configureCanaryDatabase(): void
    {
        $path = tempnam(sys_get_temp_dir(), 'oteryn-character-profile-');
        self::assertIsString($path);
        $this->canaryDatabasePath = $path;

        config()->set('database.connections.canary', [
            'driver' => 'sqlite',
            'database' => $path,
            'prefix' => '',
            'foreign_key_constraints' => true,
        ]);
        DB::purge('canary');

        Schema::connection('canary')->create('players', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->unsignedInteger('account_id');
            $table->integer('level');
            $table->integer('vocation');
            $table->bigInteger('experience')->default(0);
            $table->integer('maglevel')->default(0);
            $table->bigInteger('lastlogin')->default(0);
            $table->bigInteger('lastlogout')->default(0);
            $table->integer('boss_points')->default(0);
            $table->string('comment')->default('');
            $table->integer('skill_fist')->default(10);
            $table->integer('skill_club')->default(10);
            $table->integer('skill_sword')->default(10);
            $table->integer('skill_axe')->default(10);
            $table->integer('skill_dist')->default(10);
            $table->integer('skill_shielding')->default(10);
            $table->integer('skill_fishing')->default(10);
            $table->bigInteger('deletion')->default(0);
        });
        Schema::connection('canary')->create('guilds', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->integer('ownerid')->default(0);
            $table->integer('level')->default(1);
            $table->bigInteger('creationdata')->default(0);
            $table->text('motd')->default('');
            $table->integer('points')->default(0);
        });
        Schema::connection('canary')->create('guild_ranks', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->integer('guild_id');
            $table->string('name');
            $table->integer('level');
        });
        Schema::connection('canary')->create('guild_membership', function (Blueprint $table): void {
            $table->integer('player_id')->primary();
            $table->integer('guild_id');
            $table->integer('rank_id');
            $table->string('nick')->default('');
        });
    }

    private function seedCharacters(): void
    {
        DB::connection('canary')->table('players')->insert([
            [
                'id' => 1,
                'name' => 'Alpha Knight',
                'account_id' => 7001,
                'level' => 120,
                'vocation' => 4,
                'lastlogin' => 1_800_000_000,
                'lastlogout' => 1_799_999_000,
                'comment' => 'Alpha',
                'deletion' => 0,
            ],
            [
                'id' => 2,
                'name' => 'Beta Druid',
                'account_id' => 7001,
                'level' => 90,
                'vocation' => 2,
                'lastlogin' => 0,
                'lastlogout' => 0,
                'comment' => 'Beta',
                'deletion' => 0,
            ],
        ]);
    }

    /**
     * @param  callable(): string  $firstOperation
     * @param  callable(): string  $secondOperation
     * @return list<string>
     */
    private function racePair(callable $firstOperation, callable $secondOperation): array
    {
        $directory = sys_get_temp_dir().'/oteryn-character-profile-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        $children = [];
        $operations = [$firstOperation, $secondOperation];

        for ($index = 0; $index < 2; $index++) {
            $pid = pcntl_fork();

            if ($pid === -1) {
                self::fail('Unable to fork character-profile concurrency test process.');
            }

            if ($pid === 0) {
                DB::disconnect();
                DB::purge();
                DB::disconnect('canary');
                DB::purge('canary');
                file_put_contents($directory.'/ready-'.$index, '1');

                while (! file_exists($directory.'/start')) {
                    usleep(1000);
                }

                try {
                    $result = $operations[$index]();
                } catch (Throwable $exception) {
                    $result = 'error:'.$exception::class;
                }

                file_put_contents($directory.'/result-'.$index, $result);
                exit(0);
            }

            $children[] = $pid;
        }

        $deadline = microtime(true) + 10;

        while ((! file_exists($directory.'/ready-0') || ! file_exists($directory.'/ready-1'))
            && microtime(true) < $deadline
        ) {
            usleep(1000);
        }

        self::assertFileExists($directory.'/ready-0');
        self::assertFileExists($directory.'/ready-1');
        file_put_contents($directory.'/start', '1');

        foreach ($children as $pid) {
            $status = null;
            pcntl_waitpid($pid, $status);

            if (! is_int($status)) {
                self::fail('Child process status was not an integer.');
            }

            self::assertTrue(pcntl_wifexited($status));
            self::assertSame(0, pcntl_wexitstatus($status));
        }

        $results = [];

        for ($index = 0; $index < 2; $index++) {
            $resultPath = $directory.'/result-'.$index;
            self::assertFileExists($resultPath);
            $result = file_get_contents($resultPath);
            self::assertIsString($result);
            $results[] = $result;
        }

        foreach (glob($directory.'/*') ?: [] as $path) {
            unlink($path);
        }
        rmdir($directory);
        DB::purge();
        DB::reconnect();

        return $results;
    }
}
