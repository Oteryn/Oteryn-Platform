<?php

namespace Tests\Feature\GameAuth\Concurrency;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\GameAuth\NativeLogin\NativeAdmissionAttempt;
use App\GameAuth\NativeLogin\NativeAdmissionAttempts;
use App\GameAuth\NativeLogin\NativeAdmissionRequest;
use App\GameAuth\NativeLogin\NativeAdmissionScope;
use App\GameAuth\NativeLogin\NativeAdmissionScopeResolver;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\NativeLogin\NativeLoginRefused;
use App\GameAuth\NativeLogin\RedeemedNativeAccount;
use App\GameAuth\Tickets\GameLoginTicket;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;
use Throwable;

/**
 * Contract §16 "concurrent redeem, one winner" on real InnoDB with independent processes. The
 * scope resolver holds the ticket row lock long enough for the other request to take its gap lock
 * on the missing attempt_ref row first, so the winner's attempt insert deterministically deadlocks
 * (1213) against the waiting loser. The loser must re-read the committed attempt, never grant twice.
 * A lock conflict that repeats on the one re-read is the retryable NATIVE_LOGIN_UNAVAILABLE.
 */
final class NativeGameTicketConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    private const ATTEMPT = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';

    private const OTHER_ATTEMPT = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a70';

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB concurrency workflow with pcntl.');
        }

        $this->directory = sys_get_temp_dir().'/oteryn-native-login-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($this->directory.'/witness', 0700, true));
        $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        file_put_contents($this->directory.'/current.key', rtrim(strtr(base64_encode($seed), '+/', '-_'), '=')."\n");
        chmod($this->directory.'/current.key', 0600);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->directory.'/witness',
            'game-auth.native_evidence.fresh_key_purpose' => 'fresh_admission',
            'game-auth.native_evidence.clock_uncertainty_seconds' => 1,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.grant_ttl_seconds' => 20,
            'game-auth.native_admission.signing_key_file' => $this->directory.'/current.key',
            'game-auth.native_admission.signing_key_id' => 'admission-1',
        ]);
        $this->app->make(NativeSigningTrustRegistry::class)->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            'fresh_admission',
            'admission-1',
            sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($seed)),
        );
        $this->app->instance(NativeAdmissionScopeResolver::class, new class implements NativeAdmissionScopeResolver
        {
            public function resolve(RedeemedNativeAccount $account, NativeAdmissionRequest $request): NativeAdmissionScope
            {
                usleep(400_000);

                return new NativeAdmissionScope(
                    characterId: $request->characterId,
                    worldId: '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7d',
                    channelId: '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7e',
                    routeRevision: 'rt.3.0123456789abcdef0123456789abcdef',
                    runtimeObservationRevision: 'obs.42',
                    scopeOwnershipGeneration: '5',
                    rulesetRevision: 'ruleset.1',
                    contentRevision: 'content.1',
                    mapRevision: 'map.1',
                    worldPolicyRevision: 'policy.1',
                    offerRevision: 'offer.1',
                );
            }
        });
    }

    protected function tearDown(): void
    {
        if (isset($this->directory)) {
            // The redemption migration refuses rollback while native evidence exists.
            DB::table('native_admission_attempts')->delete();
            DB::table('game_login_tickets')->delete();
            foreach (glob($this->directory.'/{,witness/}*', GLOB_BRACE) ?: [] as $path) {
                is_file($path) && @unlink($path);
            }
            @rmdir($this->directory.'/witness');
            @rmdir($this->directory);
        }

        parent::tearDown();
    }

    public function test_concurrent_redeems_of_one_ticket_with_different_attempt_refs_grant_once(): void
    {
        $ticket = $this->ticket();

        $results = $this->race(
            fn (): string => $this->admit($ticket, self::ATTEMPT),
            fn (): string => $this->admit($ticket, self::OTHER_ATTEMPT),
        );

        $granted = array_values(array_filter($results, fn (string $result): bool => str_starts_with($result, 'granted:')));
        self::assertCount(1, $granted, implode(', ', $results));
        self::assertContains('NATIVE_LOGIN_TICKET_REJECTED', $results);
        $attempt = NativeAdmissionAttempt::query()->sole();
        self::assertSame($attempt->attempt_ref, GameLoginTicket::query()->sole()->attempt_ref);
    }

    public function test_concurrent_identical_requests_for_one_attempt_ref_return_the_same_grant(): void
    {
        $ticket = $this->ticket();

        $results = $this->race(
            fn (): string => $this->admit($ticket, self::ATTEMPT),
            fn (): string => $this->admit($ticket, self::ATTEMPT),
        );

        self::assertStringStartsWith('granted:', $results[0], implode(', ', $results));
        self::assertSame($results[0], $results[1]);
        self::assertSame(self::ATTEMPT, NativeAdmissionAttempt::query()->sole()->attempt_ref);
    }

    public function test_concurrent_changed_request_for_one_attempt_ref_is_an_attempt_conflict(): void
    {
        $ticket = $this->ticket();

        $results = $this->race(
            fn (): string => $this->admit($ticket, self::ATTEMPT),
            fn (): string => $this->admit($ticket, self::ATTEMPT, '0.1.0+def456'),
        );

        $granted = array_values(array_filter($results, fn (string $result): bool => str_starts_with($result, 'granted:')));
        self::assertCount(1, $granted, implode(', ', $results));
        self::assertContains('NATIVE_LOGIN_ATTEMPT_CONFLICT', $results);
        self::assertSame(self::ATTEMPT, NativeAdmissionAttempt::query()->sole()->attempt_ref);
    }

    public function test_a_repeated_lock_conflict_on_the_re_read_is_retryable_unavailable_and_grants_nothing(): void
    {
        $ticket = $this->ticket();

        // Another transaction holds the ticket row across both of the request's lock waits, so the
        // first attempt and its one re-read each end in ER_LOCK_WAIT_TIMEOUT (1205).
        $results = $this->race(
            fn (): string => $this->holdTicketRow(5),
            fn (): string => $this->afterTicketRowLocked(fn (): string => $this->admit($ticket, self::ATTEMPT)),
        );

        self::assertSame(['held', 'NATIVE_LOGIN_UNAVAILABLE'], $results);
        self::assertSame(0, NativeAdmissionAttempt::query()->count());
        $stored = GameLoginTicket::query()->sole();
        self::assertNull($stored->used_at);
        self::assertNull($stored->attempt_ref);

        // Nothing committed, so the same request succeeds once the lock is released.
        self::assertStringStartsWith('granted:', $this->admit($ticket, self::ATTEMPT));
        self::assertSame(self::ATTEMPT, NativeAdmissionAttempt::query()->sole()->attempt_ref);
    }

    private function holdTicketRow(int $seconds): string
    {
        DB::transaction(function () use ($seconds): void {
            DB::table('game_login_tickets')->lockForUpdate()->get();
            file_put_contents($this->directory.'/ticket-locked', '1');
            sleep($seconds);
        });

        return 'held';
    }

    /**
     * @param  callable(): string  $operation
     */
    private function afterTicketRowLocked(callable $operation): string
    {
        $deadline = microtime(true) + 10;
        while (! file_exists($this->directory.'/ticket-locked')) {
            if (microtime(true) > $deadline) {
                return 'error:ticket-row-not-locked';
            }
            usleep(1000);
        }
        DB::statement('SET SESSION innodb_lock_wait_timeout = 1');

        return $operation();
    }

    private function admit(string $ticket, string $attemptRef, string $build = '0.1.0+abc123'): string
    {
        $request = new NativeAdmissionRequest($ticket, $attemptRef, self::CHARACTER, null, [
            'client_build' => $build,
            'client_platform' => 'windows',
            'transports' => [['protocol_major' => 1, 'transport_profile' => 1, 'alpn' => 'oteryn-game/1']],
        ]);
        try {
            return 'granted:'.$this->app->make(NativeAdmissionAttempts::class)->admit($request)->token;
        } catch (NativeLoginRefused $refused) {
            return $refused->error->value;
        }
    }

    private function ticket(): string
    {
        $identity = Identity::query()->create([
            'email' => 'native-race@example.test',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ])->refresh();
        $ticket = bin2hex(random_bytes(32));
        GameLoginTicket::query()->create([
            'ticket_hash' => hash('sha256', $ticket),
            'identity_id' => $identity->id,
            'canary_account_id' => null,
            'account_id' => $identity->account_id,
            'audience' => NativeGameLoginTickets::AUDIENCE,
            'security_generation' => $identity->game_auth_generation,
            'native_security_generation' => $identity->native_security_generation,
            'expires_at' => now()->addSeconds(60),
        ]);

        return $ticket;
    }

    /**
     * Runs both operations in forked processes with their own connections, released together.
     *
     * @param  callable(): string  $first
     * @param  callable(): string  $second
     * @return list<string>
     */
    private function race(callable $first, callable $second): array
    {
        $barrier = $this->directory.'/race';
        self::assertTrue(mkdir($barrier, 0700));
        $children = [];
        foreach ([$first, $second] as $index => $operation) {
            $pid = pcntl_fork();
            if ($pid === -1) {
                self::fail('Unable to fork concurrency test process.');
            }
            if ($pid === 0) {
                DB::disconnect();
                DB::purge();
                file_put_contents($barrier.'/ready-'.$index, '1');
                while (! file_exists($barrier.'/start')) {
                    usleep(1000);
                }
                try {
                    $result = $operation();
                } catch (Throwable $exception) {
                    $result = 'error:'.$exception::class;
                }
                file_put_contents($barrier.'/result-'.$index, $result);
                exit(0);
            }
            $children[] = $pid;
        }

        $deadline = microtime(true) + 10;
        while ((! file_exists($barrier.'/ready-0') || ! file_exists($barrier.'/ready-1')) && microtime(true) < $deadline) {
            usleep(1000);
        }
        file_put_contents($barrier.'/start', '1');
        foreach ($children as $pid) {
            $status = null;
            pcntl_waitpid($pid, $status);
            if (! is_int($status)) {
                self::fail('Child process status was not an integer.');
            }
            self::assertTrue(pcntl_wifexited($status) && pcntl_wexitstatus($status) === 0);
        }

        $results = [];
        foreach ([0, 1] as $index) {
            $result = file_get_contents($barrier.'/result-'.$index);
            self::assertIsString($result);
            $results[] = $result;
        }
        array_map('unlink', glob($barrier.'/*') ?: []);
        rmdir($barrier);
        DB::purge();
        DB::reconnect();

        return $results;
    }
}
