<?php

namespace Tests\Feature\GameAuth\Concurrency;

use App\Accounts\Models\IdentityCanaryAccount;
use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceHighWaterWitness;
use App\GameAuth\NativeEvidence\NativeEvidenceNamespace;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\GameAuth\OAuth\IssueGameLoginTicketFromOAuth;
use App\GameAuth\OAuth\NativeOAuthClientManager;
use App\GameAuth\OAuth\OAuthBootstrapDenied;
use App\GameAuth\Tickets\GameLoginTicket;
use App\GameAuth\Tickets\GameLoginTicketDenied;
use App\GameAuth\Tickets\IssueGameLoginTicket;
use App\GameAuth\Tickets\RedeemGameLoginTicket;
use App\Identity\Actions\RevokeIdentityGameAuthorizations;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Laravel\Passport\RefreshToken;
use Laravel\Passport\Token;
use Symfony\Component\Process\Process;
use Tests\TestCase;
use Throwable;

final class GameTicketConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB concurrency workflow with pcntl.');
        }
    }

    public function test_exactly_one_concurrent_ticket_redeem_succeeds_across_independent_processes(): void
    {
        $identity = $this->createIdentityWithReadyBinding(1001);
        $issued = $this->app->make(IssueGameLoginTicket::class)->execute($identity);

        $results = $this->race(function () use ($issued): string {
            try {
                $this->app->make(RedeemGameLoginTicket::class)
                    ->execute($issued->ticket, 'oteryn-game-gateway');

                return 'success';
            } catch (GameLoginTicketDenied) {
                return 'denied';
            }
        });

        sort($results);
        self::assertSame(['denied', 'success'], $results);
        self::assertNotNull(GameLoginTicket::query()->firstOrFail()->used_at);
    }

    public function test_exactly_one_concurrent_oauth_exchange_mints_a_ticket_and_revokes_token_family(): void
    {
        $identity = $this->createIdentityWithReadyBinding(1001);
        $client = $this->app->make(NativeOAuthClientManager::class)->ensure();
        $accessTokenId = $this->createBootstrapTokenFamily($identity, $this->clientId($client->getKey()));

        $results = $this->race(function () use ($identity, $accessTokenId): string {
            try {
                $freshIdentity = Identity::query()->findOrFail($identity->id);
                $this->app->make(IssueGameLoginTicketFromOAuth::class)
                    ->execute($freshIdentity, $accessTokenId);

                return 'success';
            } catch (OAuthBootstrapDenied|GameLoginTicketDenied) {
                return 'denied';
            }
        });

        sort($results);
        self::assertSame(['denied', 'success'], $results);
        self::assertSame(1, GameLoginTicket::query()->count());
        $accessToken = Token::query()->whereKey($accessTokenId)->firstOrFail();
        self::assertTrue((bool) $accessToken->getAttribute('revoked'));
        self::assertTrue((bool) RefreshToken::query()
            ->where('access_token_id', $accessTokenId)
            ->value('revoked'));
    }

    public function test_game_authorization_revocation_serializes_against_oauth_bootstrap(): void
    {
        $identity = $this->createIdentityWithReadyBinding(1001);
        $client = $this->app->make(NativeOAuthClientManager::class)->ensure();
        $accessTokenId = $this->createBootstrapTokenFamily($identity, $this->clientId($client->getKey()));

        $results = $this->racePair(
            function () use ($identity, $accessTokenId): string {
                try {
                    $freshIdentity = Identity::query()->findOrFail($identity->id);
                    $this->app->make(IssueGameLoginTicketFromOAuth::class)
                        ->execute($freshIdentity, $accessTokenId);

                    return 'bootstrap';
                } catch (OAuthBootstrapDenied|GameLoginTicketDenied) {
                    return 'bootstrap-denied';
                }
            },
            function () use ($identity): string {
                $freshIdentity = Identity::query()->findOrFail($identity->id);
                $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($freshIdentity);

                return 'revoked';
            },
        );

        self::assertContains('revoked', $results);
        self::assertCount(1, array_intersect($results, ['bootstrap', 'bootstrap-denied']));

        $freshIdentity = Identity::query()->findOrFail($identity->id);
        self::assertSame(1, $freshIdentity->game_auth_generation);
        $accessToken = Token::query()->whereKey($accessTokenId)->firstOrFail();
        self::assertTrue((bool) $accessToken->getAttribute('revoked'));
        self::assertTrue((bool) RefreshToken::query()
            ->where('access_token_id', $accessTokenId)
            ->value('revoked'));
        self::assertLessThanOrEqual(1, GameLoginTicket::query()->count());

        $ticket = GameLoginTicket::query()->first();

        if ($ticket instanceof GameLoginTicket) {
            self::assertSame(0, $ticket->security_generation);
            self::assertLessThan($freshIdentity->game_auth_generation, $ticket->security_generation);
        }
    }

    public function test_native_evidence_account_reconciliation_survives_outer_rollback_and_process_restart(): void
    {
        $this->withNativeEvidenceDirectory(function (string $directory): void {
            $identity = Identity::query()->create([
                'email' => 'native-restart-account@example.test',
                'password' => Hash::make('Correct-Horse-9!Battery'),
            ]);
            $accountId = $identity->account_id;

            $namespace = NativeEvidenceNamespace::accountState($accountId);
            $witness = $this->app->make(NativeEvidenceHighWaterWitness::class);
            $witness->withNamespace($namespace, function (?int $floor, \Closure $advance): void {
                self::assertNull($floor);
                $advance(1);
            });

            DB::beginTransaction();
            try {
                $this->app->make(RevokeIdentityGameAuthorizations::class)->execute($identity->refresh());
                self::assertSame(2, $this->databaseInt(
                    Identity::query()->where('account_id', $accountId)->value('native_security_generation'),
                    'native security generation after revoked transaction',
                ));
            } finally {
                DB::rollBack();
            }

            DB::purge();
            DB::reconnect();
            self::assertSame(1, $this->databaseInt(
                Identity::query()->where('account_id', $accountId)->value('native_security_generation'),
                'native security generation after rollback',
            ));
            self::assertSame(2, $this->app->make(NativeEvidenceHighWaterWitness::class)->peek($namespace));

            $processes = [
                $this->nativeEvidenceReconcileProcess($directory, '--account-id='.$accountId),
                $this->nativeEvidenceReconcileProcess($directory, '--account-id='.$accountId),
            ];
            $this->runProcessesConcurrently($processes);

            DB::purge();
            DB::reconnect();
            self::assertSame(2, $this->databaseInt(
                Identity::query()->where('account_id', $accountId)->value('native_security_generation'),
                'native security generation after restart reconciliation',
            ));
            self::assertSame(2, $this->app->make(NativeEvidenceHighWaterWitness::class)->peek($namespace));
        });
    }

    public function test_native_evidence_signing_trust_recovery_survives_outer_rollback_and_process_restart(): void
    {
        $this->withNativeEvidenceDirectory(function (string $directory): void {
            $registry = $this->app->make(NativeSigningTrustRegistry::class);
            $keyOne = str_repeat("\x01", 32);
            $keyTwo = str_repeat("\x02", 32);

            $registry->publishTrustedKey(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                'fresh_admission',
                'key-1',
                $keyOne,
            );

            DB::beginTransaction();
            try {
                $registry->revokeProfile(
                    NativeEvidenceContract::FRESH_ISSUER,
                    NativeEvidenceContract::FRESH_PROFILE,
                    'fresh_admission',
                );
                self::assertNotNull(DB::table('native_game_signing_trust_profiles')
                    ->where('profile_version', 1)
                    ->value('revoked_at'));
            } finally {
                DB::rollBack();
            }

            DB::purge();
            DB::reconnect();
            self::assertEquals(1, DB::table('native_game_signing_trust_profiles')
                ->where('profile_version', 1)
                ->value('issuer_revision'));
            self::assertNull(DB::table('native_game_signing_trust_profiles')
                ->where('profile_version', 1)
                ->value('revoked_at'));

            $processes = [
                $this->nativeEvidenceReconcileProcess($directory, '--trust=fresh'),
                $this->nativeEvidenceReconcileProcess($directory, '--trust=fresh'),
            ];
            $this->runProcessesConcurrently($processes);

            DB::purge();
            DB::reconnect();
            self::assertEquals(2, DB::table('native_game_signing_trust_profiles')
                ->where('profile_version', 1)
                ->value('issuer_revision'));
            self::assertNotNull(DB::table('native_game_signing_trust_profiles')
                ->where('profile_version', 1)
                ->value('revoked_at'));

            $next = $this->app->make(NativeSigningTrustRegistry::class)->publishNextProfileVersion(
                NativeEvidenceContract::FRESH_ISSUER,
                NativeEvidenceContract::FRESH_PROFILE,
                'fresh_admission',
                'key-2',
                $keyTwo,
            );
            self::assertEquals(1, $next->key_revision);
            self::assertSame(2, DB::table('native_game_signing_trust_profiles')->count());
            self::assertEquals(3, DB::table('native_game_signing_trust_profiles')
                ->where('profile_version', 2)
                ->value('issuer_revision'));

            DB::table('native_game_signing_trust_key_versions')
                ->whereIn('profile_id', DB::table('native_game_signing_trust_profiles')
                    ->select('id')
                    ->where('profile_version', '>', 1))
                ->delete();
            DB::table('native_game_signing_trust_profiles')
                ->where('profile_version', '>', 1)
                ->delete();
        });
    }

    /**
     * @param  callable(): string  $operation
     * @return list<string>
     */
    private function race(callable $operation): array
    {
        return $this->racePair($operation, $operation);
    }

    /**
     * @param  callable(): string  $firstOperation
     * @param  callable(): string  $secondOperation
     * @return list<string>
     */
    private function racePair(callable $firstOperation, callable $secondOperation): array
    {
        $directory = sys_get_temp_dir().'/oteryn-game-auth-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        $children = [];
        $operations = [$firstOperation, $secondOperation];

        for ($index = 0; $index < 2; $index++) {
            $pid = pcntl_fork();

            if ($pid === -1) {
                self::fail('Unable to fork concurrency test process.');
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
            $path = $directory.'/result-'.$index;
            self::assertFileExists($path);
            $result = file_get_contents($path);
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

    /** @param callable(string): void $callback */
    private function withNativeEvidenceDirectory(callable $callback): void
    {
        $directory = sys_get_temp_dir().'/oteryn-native-evidence-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($directory, 0700));
        config([
            'game-auth.native_evidence.source_authority' => 'platform',
            'game-auth.native_evidence.activated' => true,
            'game-auth.native_evidence.high_water_directory' => $directory,
            'game-auth.native_evidence.fresh_account_purpose' => 'platform_security',
            'game-auth.native_evidence.fresh_account_scope' => 'fresh_admission',
            'game-auth.native_evidence.fresh_key_purpose' => 'fresh_admission',
            'game-auth.native_evidence.clock_uncertainty_seconds' => 0,
            'game-auth.native_evidence.requests_per_minute' => 120,
        ]);

        try {
            $callback($directory);
        } finally {
            config(['game-auth.native_evidence.activated' => false]);
            DB::purge();
            DB::reconnect();
            foreach (glob($directory.'/*') ?: [] as $path) {
                @unlink($path);
            }
            @rmdir($directory);
        }
    }

    private function nativeEvidenceReconcileProcess(string $directory, string $option): Process
    {
        $process = new Process(
            [PHP_BINARY, base_path('artisan'), 'game-auth:native-evidence:reconcile', $option],
            base_path(),
            [
                'GAME_AUTH_NATIVE_EVIDENCE_SOURCE_AUTHORITY' => 'platform',
                'GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED' => 'true',
                'GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY' => $directory,
                'GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_PURPOSE' => 'platform_security',
                'GAME_AUTH_NATIVE_EVIDENCE_FRESH_ACCOUNT_SCOPE' => 'fresh_admission',
                'GAME_AUTH_NATIVE_EVIDENCE_FRESH_KEY_PURPOSE' => 'fresh_admission',
                'GAME_AUTH_NATIVE_EVIDENCE_CLOCK_UNCERTAINTY_SECONDS' => '0',
                'GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE' => '120',
            ],
        );
        $process->setTimeout(30);

        return $process;
    }

    /** @param list<Process> $processes */
    private function runProcessesConcurrently(array $processes): void
    {
        foreach ($processes as $process) {
            $process->start();
        }

        foreach ($processes as $process) {
            $exitCode = $process->wait();
            self::assertSame(
                0,
                $exitCode,
                "Native evidence restart process failed.\nstdout:\n{$process->getOutput()}\nstderr:\n{$process->getErrorOutput()}",
            );
        }
    }

    private function databaseInt(mixed $value, string $label): int
    {
        if (is_int($value)) {
            return $value;
        }
        if (is_string($value) && preg_match('/^[0-9]+$/', $value) === 1) {
            return (int) $value;
        }

        throw new \RuntimeException("{$label} must be a non-negative integer.");
    }

    private function createBootstrapTokenFamily(Identity $identity, string $clientId): string
    {
        $accessTokenId = Str::random(80);

        Token::query()->create([
            'id' => $accessTokenId,
            'user_id' => $identity->id,
            'client_id' => $clientId,
            'name' => null,
            'scopes' => ['game:ticket'],
            'revoked' => false,
            'game_auth_generation' => $identity->game_auth_generation,
            'expires_at' => now()->addMinutes(5),
        ]);
        RefreshToken::query()->create([
            'id' => Str::random(80),
            'access_token_id' => $accessTokenId,
            'revoked' => false,
            'expires_at' => now()->addMinutes(10),
        ]);

        return $accessTokenId;
    }

    private function clientId(mixed $key): string
    {
        if (! is_int($key) && ! is_string($key)) {
            self::fail('Native OAuth client identifier is unavailable.');
        }

        return (string) $key;
    }

    private function createIdentityWithReadyBinding(int $canaryAccountId): Identity
    {
        $identity = Identity::query()->create([
            'email' => 'person@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => $canaryAccountId,
            'provisioning_name' => 'ready_'.$identity->id,
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);

        return $identity->refresh();
    }
}
