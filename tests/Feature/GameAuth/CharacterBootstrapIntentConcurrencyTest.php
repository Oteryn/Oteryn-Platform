<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentConflict;
use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentIssuer;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;
use Throwable;

final class CharacterBootstrapIntentConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    protected function setUp(): void
    {
        parent::setUp();
        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1'
            || ! function_exists('pcntl_fork')
            || DB::connection()->getDriverName() !== 'mysql') {
            $this->markTestSkipped('Requires the dedicated MariaDB GameAuth concurrency environment with pcntl.');
        }
        config([
            'hashing.driver' => 'bcrypt',
            'game-auth.character_bootstrap_intent.ttl_seconds' => '60',
        ]);
    }

    public function test_concurrent_identical_issuance_yields_one_semantic_intent(): void
    {
        $identity = $this->identity('identical-race@example.test');
        $results = $this->race($identity->id, [$this->binding(), $this->binding()]);

        self::assertSame(['issued', 'issued'], $results);
        self::assertSame(1, DB::table('character_bootstrap_intents')->count());
        self::assertSame(1, DB::table('character_bootstrap_intent_authority')->value('last_source_revision'));
    }

    public function test_concurrent_conflicting_operation_reuse_does_not_double_issue(): void
    {
        $identity = $this->identity('conflict-race@example.test');
        $changed = $this->binding();
        $changed['target_world_id'] = '01890f4e-7c00-7000-8000-000000000003';
        $results = $this->race($identity->id, [$this->binding(), $changed]);
        sort($results);

        self::assertSame(['conflict', 'issued'], $results);
        self::assertSame(1, DB::table('character_bootstrap_intents')->count());
        self::assertSame(1, DB::table('character_bootstrap_intent_authority')->value('last_source_revision'));
    }

    /**
     * @param  list<array{target_world_id:string,profile_revision:string,ruleset_revision:string,content_revision:string,starter_template_revision:string}>  $bindings
     * @return list<string>
     */
    private function race(int $identityId, array $bindings): array
    {
        $directory = sys_get_temp_dir().'/oteryn-character-intent-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        $operationId = '01890f4e-7c00-7000-8000-000000000001';
        $children = [];

        foreach ($bindings as $index => $binding) {
            $pid = pcntl_fork();
            if ($pid === -1) {
                self::fail('Unable to fork Character bootstrap-intent test process.');
            }
            if ($pid === 0) {
                DB::disconnect();
                DB::purge();
                touch($directory.'/ready-'.$index);
                while (! file_exists($directory.'/start')) {
                    usleep(1000);
                }
                try {
                    app(CharacterBootstrapIntentIssuer::class)->issue($identityId, $operationId, $binding);
                    $result = 'issued';
                } catch (CharacterBootstrapIntentConflict) {
                    $result = 'conflict';
                } catch (Throwable $exception) {
                    $result = 'error:'.$exception->getMessage();
                }
                file_put_contents($directory.'/result-'.$index, $result);
                exit(0);
            }
            $children[] = $pid;
        }

        $deadline = microtime(true) + 10;
        while ((! file_exists($directory.'/ready-0') || ! file_exists($directory.'/ready-1')) && microtime(true) < $deadline) {
            usleep(1000);
        }
        touch($directory.'/start');
        foreach ($children as $pid) {
            $status = 0;
            pcntl_waitpid($pid, $status);
            self::assertIsInt($status);
            self::assertTrue(pcntl_wifexited($status));
            self::assertSame(0, pcntl_wexitstatus($status));
        }

        DB::purge();
        DB::reconnect();
        $results = [
            (string) file_get_contents($directory.'/result-0'),
            (string) file_get_contents($directory.'/result-1'),
        ];
        foreach (glob($directory.'/*') ?: [] as $path) {
            unlink($path);
        }
        rmdir($directory);

        return $results;
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create(['email' => $email, 'password' => Hash::make('Race-Test-9!Password')]);
    }

    /** @return array{target_world_id:string,profile_revision:string,ruleset_revision:string,content_revision:string,starter_template_revision:string} */
    private function binding(): array
    {
        return [
            'target_world_id' => '01890f4e-7c00-7000-8000-000000000002',
            'profile_revision' => 'profile-17',
            'ruleset_revision' => 'ruleset-22',
            'content_revision' => 'content-91',
            'starter_template_revision' => 'starter-4',
        ];
    }
}
