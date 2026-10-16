<?php

namespace Tests\Feature\CharacterProfiles\Concurrency;

use App\CharacterProfiles\CharacterProfilePreferenceService;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\Support\SeedsNativeAccountCharacters;
use Tests\TestCase;
use Throwable;

final class CharacterProfilePreferenceConcurrencyTest extends TestCase
{
    use DatabaseMigrations;
    use SeedsNativeAccountCharacters;

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const ALPHA = '01934f10-7c04-7001-805b-3b1122334411';

    private const BETA = '01934f10-7c04-7001-805b-3b1122334412';

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('CHARACTER_PROFILE_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB character-profile concurrency workflow with pcntl.');
        }
    }

    public function test_concurrent_main_character_selection_preserves_single_main_invariant(): void
    {
        $identity = Identity::query()->create([
            'email' => uniqid('character-profile-concurrency-', true).'@example.com',
            'password' => Hash::make('Concurrency-Character-Profile-9!'),
        ]);
        $this->enableNativeAccountCharacters();
        $this->nativeAccountCharactersLive();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight'], [self::BETA, self::WORLD, 'Beta Druid']]);

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
            ->orderBy('character_id')
            ->get();
        self::assertCount(2, $preferences);

        $mainPreferences = $preferences->filter(
            fn (CharacterProfilePreference $preference): bool => $preference->is_main_character,
        );
        self::assertCount(1, $mainPreferences);

        $mainPreference = $mainPreferences->first();
        self::assertInstanceOf(CharacterProfilePreference::class, $mainPreference);
        self::assertContains($mainPreference->character_id, [self::ALPHA, self::BETA]);
    }

    private function selectMainCharacter(int $identityId, string $characterName): string
    {
        $identity = Identity::query()->findOrFail($identityId);

        $this->app->make(CharacterProfilePreferenceService::class)->update($identity, $characterName, ['show_guild' => false, 'is_main_character' => true]);

        return 'success';
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
