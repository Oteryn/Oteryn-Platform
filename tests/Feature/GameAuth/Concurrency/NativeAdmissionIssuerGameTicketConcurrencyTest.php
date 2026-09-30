<?php

namespace Tests\Feature\GameAuth\Concurrency;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use App\GameAuth\NativeLogin\NativeAdmissionAttempt;
use App\GameAuth\NativeLogin\NativeAdmissionAttempts;
use App\GameAuth\NativeLogin\NativeAdmissionWire;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\GameAuth\NativeLogin\NativeLoginRefused;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRefused;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentIngestion;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentReport;
use App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings;
use App\GameAuth\Tickets\GameLoginTicket;
use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use App\Identity\Models\Identity;
use Illuminate\Database\Events\QueryExecuted;
use Illuminate\Foundation\Testing\DatabaseMigrations;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;
use Throwable;

/**
 * Native issuance against a concurrent scope-assignment epoch raise on real InnoDB with independent
 * processes (login contract §6.2, §7.2 restore reset, §7.4). The issuer takes the shared epoch lock before
 * any non-locking read, so the first operation pauses while it holds the epoch lock and the second
 * deterministically waits: a raise waits for an in-flight issuance without deadlock, and an issuance that
 * waited for a raise sees it and routes nowhere, leaving the ticket unused. The class name registers it
 * in the existing `--filter=GameTicketConcurrencyTest` MariaDB step.
 */
final class NativeAdmissionIssuerGameTicketConcurrencyTest extends TestCase
{
    use DatabaseMigrations;

    private const OPS = 'CN=oteryn-game-ops.ownership-authority';

    private const NODE = 'CN=node-a.runtime-status';

    private const PURPOSE = 'fresh_admission';

    private const ATTEMPT = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';

    private const CHARACTER = '0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c';

    private string $directory;

    private string $worldId;

    /** @var array<string, string> channel key => ChannelId */
    private array $channels = [];

    protected function setUp(): void
    {
        parent::setUp();

        if (getenv('GAME_AUTH_CONCURRENCY_TEST') !== '1' || ! function_exists('pcntl_fork')) {
            $this->markTestSkipped('Requires the dedicated MariaDB concurrency workflow with pcntl.');
        }

        $this->directory = sys_get_temp_dir().'/oteryn-native-issuer-'.bin2hex(random_bytes(8));
        self::assertTrue(mkdir($this->directory.'/witness', 0700, true));
        $seed = random_bytes(SODIUM_CRYPTO_SIGN_SEEDBYTES);
        file_put_contents($this->directory.'/current.key', rtrim(strtr(base64_encode($seed), '+/', '-_'), '=')."\n");
        chmod($this->directory.'/current.key', 0600);

        $worldRow = GameWorld::query()->create([
            'slug' => 'native-entry',
            'name' => 'Native entry',
            'region' => 'TEST',
            'status' => GameWorldStatus::Maintenance,
            'login_enabled' => false,
            'game_host' => '127.0.0.1',
            'game_port' => 7172,
        ])->id;
        $registry = new NativeTopologyRegistry;
        foreach (['alpha', 'beta'] as $key) {
            $receipt = $registry->issueForPreproduction($worldRow, $key);
            $this->worldId = $receipt->worldId;
            $this->channels[$key] = $receipt->channelId;
        }
        $alpha = $registry->publishRouteForPreproduction($worldRow, 'alpha', 'game-a.example.invalid', 7172, 'game-a.example.invalid', true);
        $scopes = array_map(fn (string $channel): string => $this->worldId.'/'.$channel, array_values($this->channels));

        config([
            'game-auth.native_evidence.high_water_directory' => $this->directory.'/witness',
            'game-auth.native_evidence.fresh_key_purpose' => self::PURPOSE,
            'game-auth.native_evidence.clock_uncertainty_seconds' => 1,
            'game-auth.native_admission.enabled' => true,
            'game-auth.native_admission.grant_ttl_seconds' => 20,
            'game-auth.native_admission.signing_key_file' => $this->directory.'/current.key',
            'game-auth.native_admission.signing_key_id' => 'admission-1',
            'game-auth.native_admission.unverified_character_ownership' => true,
            'game-auth.native_admission.unverified_character_world_id' => $this->worldId,
            'game-auth.native_runtime_status.enabled' => true,
            'game-auth.native_runtime_status.identities' => [self::NODE => $scopes],
            'game-auth.native_runtime_status.freshness_seconds' => 15,
            'game-auth.native_runtime_status.clock_uncertainty_seconds' => 1,
            'game-auth.native_scope_assignment.enabled' => true,
            'game-auth.native_scope_assignment.identities' => [self::OPS => $scopes],
        ]);
        $this->app->make(NativeSigningTrustRegistry::class)->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            'admission-1',
            sodium_crypto_sign_publickey(sodium_crypto_sign_seed_keypair($seed)),
        );

        self::assertSame('accepted', $this->assign('alpha', '1', '3'));
        $this->report($alpha->routeRevision);
    }

    protected function tearDown(): void
    {
        if (isset($this->directory)) {
            // Explicit disposal of this isolated fixture so the framework's migrate-down can run.
            foreach (['native_admission_attempts', 'game_login_tickets', 'native_runtime_status_reports', 'native_scope_assignments', 'game_channels'] as $table) {
                DB::table($table)->delete();
            }
            DB::table('game_worlds')->update(['world_id' => null]);
            foreach (glob($this->directory.'/{,witness/}*', GLOB_BRACE) ?: [] as $path) {
                is_file($path) && @unlink($path);
            }
            @rmdir($this->directory.'/witness');
            @rmdir($this->directory);
        }

        parent::tearDown();
    }

    public function test_an_epoch_raise_waits_for_an_in_flight_issuance_without_deadlock(): void
    {
        $ticket = $this->ticket();

        $results = $this->race(
            fn (): string => $this->pausingAfterEpochLock(fn (): string => $this->issue($ticket)),
            fn (): string => $this->afterEpochLocked(fn (): string => $this->assign('beta', '2', '1')),
        );

        self::assertSame(['granted', 'accepted'], $results);
        $attempt = NativeAdmissionAttempt::query()->sole();
        self::assertSame($this->channels['alpha'], $attempt->channel_id);
        self::assertNotNull(GameLoginTicket::query()->sole()->used_at);
        // The raise committed after the grant: the old epoch no longer routes (Game admission rejects the grant).
        $readModel = $this->app->make(NativeRuntimeStatusReadModel::class);
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $readModel->evidence($this->worldId, $this->channels['alpha'], now()->getTimestamp()));
    }

    public function test_an_issuance_that_waited_for_an_epoch_raise_sees_it_and_leaves_the_ticket_unused(): void
    {
        $ticket = $this->ticket();

        $results = $this->race(
            fn (): string => $this->pausingAfterEpochLock(fn (): string => $this->assign('beta', '2', '1')),
            fn (): string => $this->afterEpochLocked(fn (): string => $this->issue($ticket)),
        );

        self::assertSame(['accepted', 'refused:NATIVE_LOGIN_ROUTE_UNAVAILABLE'], $results);
        self::assertSame(0, NativeAdmissionAttempt::query()->count());
        self::assertNull(GameLoginTicket::query()->sole()->used_at);
    }

    private function issue(string $ticket): string
    {
        $body = json_encode([
            'protocol_version' => 2,
            'game_login_ticket' => $ticket,
            'attempt_ref' => self::ATTEMPT,
            'character_id' => self::CHARACTER,
            'channel_id' => null,
            'offer' => [
                'client_build' => '0.1.0+abc123',
                'client_platform' => 'windows',
                'transports' => [['protocol_major' => 1, 'transport_profile' => 1, 'alpn' => 'oteryn-game/1']],
            ],
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        try {
            $this->app->make(NativeAdmissionAttempts::class)->admit(NativeAdmissionWire::decode($body));

            return 'granted';
        } catch (NativeLoginRefused $refused) {
            return 'refused:'.$refused->error->value;
        }
    }

    private function assign(string $key, string $epoch, string $generation): string
    {
        $settings = NativeScopeAssignmentSettings::current();
        $runtime = NativeRuntimeStatusSettings::current();
        self::assertNotNull($settings);
        self::assertNotNull($runtime);

        try {
            return $this->app->make(NativeScopeAssignmentIngestion::class)->ingest($settings, $runtime, self::OPS, NativeScopeAssignmentReport::fromWire(json_encode([
                'contract_version' => 1,
                'operation' => 'ReportScopeAssignmentV1',
                'assignment_epoch' => $epoch,
                'world_id' => $this->worldId,
                'channel_id' => $this->channels[$key],
                'ownership_generation' => $generation,
                'node_identity' => self::NODE,
                'assigned_at' => (string) (now()->getTimestamp() - 10),
            ], JSON_THROW_ON_ERROR)));
        } catch (NativeRuntimeStatusRefused $refused) {
            return 'refused:'.$refused->status;
        }
    }

    private function report(string $routeRevision): void
    {
        $now = now()->getTimestamp();
        DB::table('native_runtime_status_reports')->insert([
            'world_id' => $this->worldId,
            'channel_id' => $this->channels['alpha'],
            'node_identity' => self::NODE,
            'source_authority' => 'oteryn-game',
            'node_id' => '01934f10-7c04-7001-805b-3b1122334401',
            'assignment_epoch' => '1',
            'scope_ownership_generation' => '3',
            'source_revision' => '7',
            'decision_identity' => 'runtime-readiness:0a:3:7:true',
            'ready' => true,
            'published_at' => $now,
            'observed_at' => $now,
            'protocol_major' => 1,
            'transport_profile' => 1,
            'route_revision' => $routeRevision,
            'runtime_observation_revision' => 'observation-1',
            'ruleset_revision' => 'ruleset-1',
            'content_revision' => 'content-1',
            'map_revision' => 'map-1',
            'world_policy_revision' => 'policy-1',
            'offer_revision' => 'offer-1',
            'content_digest' => str_repeat('0', 64),
            'invalid' => false,
        ]);
    }

    private function ticket(): string
    {
        $identity = Identity::query()->create(['email' => 'native-issuer-race@example.test', 'password' => Hash::make('Correct-Horse-9!Battery')])->refresh();
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
     * Holds the epoch lock for 700 ms after taking it, inside the operation's transaction.
     *
     * @param  callable(): string  $operation
     */
    private function pausingAfterEpochLock(callable $operation): string
    {
        DB::listen(function (QueryExecuted $query): void {
            if (str_contains($query->sql, 'native_assignment_epoch_locks') && ! file_exists($this->directory.'/epoch-locked')) {
                file_put_contents($this->directory.'/epoch-locked', '1');
                usleep(700_000);
            }
        });

        return $operation();
    }

    /** @param callable(): string $operation */
    private function afterEpochLocked(callable $operation): string
    {
        $deadline = microtime(true) + 10;
        while (! file_exists($this->directory.'/epoch-locked')) {
            if (microtime(true) > $deadline) {
                return 'error:epoch-not-locked';
            }
            usleep(1000);
        }

        return $operation();
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
        $children = [];
        foreach ([$first, $second] as $index => $operation) {
            $pid = pcntl_fork();
            if ($pid === -1) {
                self::fail('Unable to fork concurrency test process.');
            }
            if ($pid === 0) {
                DB::disconnect();
                DB::purge();
                try {
                    $result = $operation();
                } catch (Throwable $exception) {
                    $result = 'error:'.$exception::class;
                }
                file_put_contents($this->directory.'/result-'.$index, $result);
                exit(0);
            }
            $children[] = $pid;
        }

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
            $result = file_get_contents($this->directory.'/result-'.$index);
            self::assertIsString($result);
            $results[] = $result;
        }
        DB::purge();
        DB::reconnect();

        return $results;
    }
}
