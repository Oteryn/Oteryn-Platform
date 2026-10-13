<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceDatabaseIdentity;
use App\GameAuth\NativeEvidence\NativeEvidenceHighWaterWitness;
use App\GameAuth\NativeEvidence\NativeEvidencePublicationFence;
use App\GameAuth\NativeEvidence\NativeEvidenceUnavailable;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Illuminate\Database\Events\QueryExecuted;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\Feature\GameAuth\Concerns\WritesAttemptEvidence;
use Tests\TestCase;

final class NativeEvidencePublicationFenceTest extends TestCase
{
    use RefreshDatabase;
    use WritesAttemptEvidence;

    private const OTHER_RUN = '99999999-8888-4777-8666-555555555555';

    private string $root;

    private string $witnessDirectory;

    private string $evidenceDirectory;

    private string $storeId;

    protected function setUp(): void
    {
        parent::setUp();

        $this->root = storage_path('framework/testing/native-publication-fence-'.bin2hex(random_bytes(6)));
        $this->witnessDirectory = $this->root.'/witness';
        $this->evidenceDirectory = $this->root.'/evidence';
        mkdir($this->witnessDirectory, 0700, true);
        mkdir($this->evidenceDirectory, 0700, true);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->witnessDirectory,
            'game-auth.native_evidence.fresh_key_purpose' => self::FIXTURE_PURPOSE,
            'game-auth.native_evidence.first_publication_fence' => 'required',
            'game-auth.native_evidence.attempt_evidence_directory' => $this->evidenceDirectory,
        ]);
        $this->storeId = (string) app(NativeEvidenceHighWaterWitness::class)->initializeStoreProvenance()->storeId;
    }

    protected function tearDown(): void
    {
        foreach (glob($this->root.'/{witness,evidence}/*', GLOB_BRACE) ?: [] as $path) {
            @unlink($path);
        }
        @rmdir($this->witnessDirectory);
        @rmdir($this->evidenceDirectory);
        @rmdir($this->root);

        parent::tearDown();
    }

    public function test_required_setting_refuses_a_first_publication_without_a_fence_and_writes_no_floor(): void
    {
        $this->expectException(NativeEvidenceUnavailable::class);
        $this->expectExceptionMessage('requires a first publication fence');

        try {
            $this->publish(null);
        } finally {
            $this->assertNothingWitnessedOrStored();
        }
    }

    public function test_a_clean_open_anchored_intent_verifies_and_reports_its_attempt(): void
    {
        $fence = $this->arrange('valid');

        $fence->bind(...$this->callArguments())->verify($this->storeId);

        self::assertSame(
            ['run_id' => self::FIXTURE_RUN, 'attempt' => 1, 'intent_sha256' => $fence->intentSha256],
            $fence->result(),
        );
    }

    #[DataProvider('refusals')]
    public function test_each_refusal_happens_before_any_floor_write(string $case): void
    {
        $fence = $this->arrange($case);

        try {
            $fence->bind(...$this->callArguments())->verify($this->storeId);
            self::fail('The fence must refuse '.$case.'.');
        } catch (NativeEvidenceUnavailable) {
            self::assertNull($fence->result());
        }

        try {
            $this->publish($fence);
            self::fail('The publication must refuse '.$case.'.');
        } catch (NativeEvidenceUnavailable) {
            $this->assertNothingWitnessedOrStored();
        }
    }

    /** @return array<string, array{string}> */
    public static function refusals(): array
    {
        return [
            'fabricated fence hash' => ['fabricated hash'],
            'init for another store id' => ['foreign init'],
            'run ending store-changed' => ['store-changed'],
            'run ending evidence-lost' => ['evidence-lost'],
            'run ending init-lost' => ['init-lost'],
            'init-intent without init or begin-refused' => ['unfinished init'],
            'identity differs from the first init' => ['identity changed'],
            'wal flush lsn below the init observation' => ['lsn below init'],
            'wal flush lsn below an earlier observation' => ['lsn below earlier'],
            'intent without its anchored line' => ['unanchored'],
            'anchored line names another seq' => ['anchored other seq'],
            'anchored line names another hash' => ['anchored other hash'],
            'anchored line without a comment id' => ['anchored no comment'],
            'intent followed by another line' => ['extra line'],
            'second open intent in another run' => ['second intent'],
            'broken hash chain in another file' => ['broken chain'],
            'blocking classification in another run' => ['blocking classification'],
            'missing evidence directory' => ['missing directory'],
            'unconfigured evidence directory' => ['unconfigured directory'],
            'intent store id differs' => ['intent store'],
            'intent scope differs' => ['intent scope'],
            'intent key id differs' => ['intent key'],
            'intent observation shows a floor' => ['intent floor'],
            'intent observation without database' => ['intent no database'],
        ];
    }

    public function test_marker_only_state_refuses_and_the_row_is_not_rebound(): void
    {
        $fence = $this->arrange('valid');
        DB::table('native_game_evidence_witness_stores')->delete();

        try {
            $this->publish($fence);
            self::fail('A marker-only state must refuse.');
        } catch (NativeEvidenceUnavailable $exception) {
            self::assertStringContainsString('does not re-bind', $exception->getMessage());
        }

        self::assertSame(0, DB::table('native_game_evidence_witness_stores')->count());
        self::assertSame(0, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame([], glob($this->witnessDirectory.'/*.floor*') ?: []);
    }

    public function test_the_fence_is_ignored_when_a_profile_exists_or_the_setting_is_off(): void
    {
        config(['game-auth.native_evidence.first_publication_fence' => 'off']);
        $this->publish(null);
        self::assertSame(1, DB::table('native_game_signing_trust_profiles')->count());

        config(['game-auth.native_evidence.first_publication_fence' => 'required']);
        $garbage = new NativeEvidencePublicationFence(self::FIXTURE_RUN, str_repeat('a', 64));
        $this->publish($garbage);

        self::assertNull($garbage->result());
        self::assertSame(1, DB::table('native_game_signing_trust_profiles')->count());
    }

    public function test_the_fenced_publication_refuses_when_the_database_has_no_postgres_identity(): void
    {
        if (DB::connection()->getDriverName() === 'pgsql') {
            self::markTestSkipped('Covered by the pgsql tests.');
        }
        $fence = $this->arrange('valid');

        try {
            $this->publish($fence);
            self::fail('A fenced publication needs a PostgreSQL identity.');
        } catch (NativeEvidenceUnavailable $exception) {
            self::assertStringContainsString('PostgreSQL database identity', $exception->getMessage());
        }

        $this->assertNothingWitnessedOrStored();
    }

    public function test_pgsql_fenced_first_publication_commits_and_reports_the_intent(): void
    {
        $this->requirePgsql();
        $live = (new NativeEvidenceDatabaseIdentity)->read();
        self::assertIsArray($live);
        $fence = $this->arrangeLive($live['identity'], $live['progress']);

        $this->publish($fence);

        self::assertSame(['run_id' => self::FIXTURE_RUN, 'attempt' => 1, 'intent_sha256' => $fence->intentSha256], $fence->result());
        self::assertSame(1, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame("1\n", file_get_contents($this->floorPath()));
    }

    #[DataProvider('livePgsqlRefusals')]
    public function test_pgsql_identity_and_progress_mismatches_refuse_before_the_floor_write(string $case): void
    {
        $this->requirePgsql();
        $live = (new NativeEvidenceDatabaseIdentity)->read();
        self::assertIsArray($live);
        $identity = $live['identity'];
        $progress = $live['progress'];
        match ($case) {
            'timeline' => $identity['timeline_id'] = $identity['timeline_id'] + 1,
            'system identifier' => $identity['system_identifier'] = '1',
            'relfilenode' => $identity['relfilenodes']['native_game_signing_trust_profiles'] = 1,
            'lsn above live' => $progress['wal_flush_lsn'] = 'FFFFFFFF/FFFFFFFF',
            'xmax above live' => $progress['xmax'] = '18446744073709551000',
        };
        $fence = $this->arrangeLive($identity, $progress);

        try {
            $this->publish($fence);
            self::fail('The publication must refuse '.$case.'.');
        } catch (NativeEvidenceUnavailable) {
            $this->assertNothingWitnessedOrStored();
        }
    }

    /** @return array<string, array{string}> */
    public static function livePgsqlRefusals(): array
    {
        return [
            'timeline id' => ['timeline'],
            'system identifier' => ['system identifier'],
            'relfilenode' => ['relfilenode'],
            'wal flush lsn above the live position' => ['lsn above live'],
            'xmax above the live snapshot' => ['xmax above live'],
        ];
    }

    #[DataProvider('swapsAfterTheBindingRow')]
    public function test_pgsql_marker_or_lock_replaced_after_the_binding_row_refuses(string $swap): void
    {
        $this->requirePgsql();
        $live = (new NativeEvidenceDatabaseIdentity)->read();
        self::assertIsArray($live);
        $fence = $this->arrangeLive($live['identity'], $live['progress']);

        DB::listen(function (QueryExecuted $query) use ($swap): void {
            if (DB::transactionLevel() < 1 || ! str_starts_with(strtolower(ltrim($query->sql)), 'select')
                || ! str_contains($query->sql, 'native_game_evidence_witness_stores')) {
                return;
            }
            if ($swap === 'marker') {
                file_put_contents($this->witnessDirectory.'/witness-store.id', str_repeat('b', 64)."\n");
            } else {
                $replacement = $this->witnessDirectory.'/replacement';
                file_put_contents($replacement, '');
                rename($replacement, $this->witnessDirectory.'/witness-store.lock');
            }
        });

        try {
            $this->publish($fence);
            self::fail('A replaced '.$swap.' must refuse.');
        } catch (NativeEvidenceUnavailable) {
            self::assertSame(0, DB::table('native_game_signing_trust_profiles')->count());
            self::assertSame([], glob($this->witnessDirectory.'/*.floor') ?: []);
        }
    }

    /** @return array<string, array{string}> */
    public static function swapsAfterTheBindingRow(): array
    {
        return ['marker differs' => ['marker'], 'store lock inode differs' => ['lock']];
    }

    /** Writes the evidence of one case and returns the fence that names its intent. */
    private function arrange(string $case): NativeEvidencePublicationFence
    {
        $other = 'c'.substr($this->storeId, 1);
        $options = match ($case) {
            'foreign init' => ['initStore' => $other],
            'identity changed' => ['intentDatabase' => $this->withIdentity(['system_identifier' => '7000000000000000002'])],
            'lsn below init' => ['intentDatabase' => $this->fixtureDatabase('0/FFFFFF', '710')],
            'unanchored' => ['skipAnchored' => true],
            'anchored other seq' => ['anchored' => ['for_seq' => 2]],
            'anchored other hash' => ['anchored' => ['for_line_sha256' => str_repeat('d', 64)]],
            'anchored no comment' => ['anchored' => ['anchor_comment_id' => null]],
            'extra line' => ['after' => [['type' => 'self-check', 'passed' => true]]],
            'intent store' => ['intent' => ['store_id' => $other]],
            'intent scope' => ['intent' => ['scope' => ['key_purpose' => 'other_purpose'] + $this->fixtureScope()]],
            'intent key' => ['intent' => ['key_id' => 'kid-2']],
            'intent floor' => ['intent' => ['observation' => ['floor' => ['value' => 1, 'temp_entries' => 0]] + $this->fixtureObservation($this->fixtureDatabase('0/2000000', '710'), $this->storeId)]],
            'intent no database' => ['intent' => ['observation' => $this->fixtureObservation(null, $this->storeId)]],
            default => [],
        };
        $hashes = $this->writeRun($this->evidenceDirectory, self::FIXTURE_RUN, $this->standardRecords($this->storeId, $options));
        $intentHash = $hashes[4];

        $extra = match ($case) {
            'store-changed', 'evidence-lost', 'init-lost' => [['type' => 'run-open'], ['type' => 'begin-refused', 'reason' => $case]],
            'unfinished init' => [['type' => 'run-open'], ['type' => 'init-intent'], ['type' => 'anchored']],
            'second intent' => [['type' => 'run-open'], ['type' => 'attempt-intent', 'attempt' => 1], ['type' => 'anchored']],
            'blocking classification' => [['type' => 'run-open'], ['type' => 'attempt-observation', 'attempt' => 1, 'classification' => 'unknown']],
            'broken chain' => [['type' => 'run-open', 'prev' => str_repeat('a', 64)]],
            'lsn below earlier' => [['type' => 'run-open', 'pre_init' => $this->fixtureObservation($this->fixtureDatabase('0/9000000', '720'), null)]],
            default => null,
        };
        if ($extra !== null) {
            $this->writeRun($this->evidenceDirectory, self::OTHER_RUN, $extra);
        }

        match ($case) {
            'fabricated hash' => $intentHash = str_repeat('e', 64),
            'missing directory' => config(['game-auth.native_evidence.attempt_evidence_directory' => $this->root.'/absent']),
            'unconfigured directory' => config(['game-auth.native_evidence.attempt_evidence_directory' => null]),
            default => null,
        };

        return new NativeEvidencePublicationFence(self::FIXTURE_RUN, $intentHash);
    }

    /**
     * @param  array<string, mixed>  $identity
     * @param  array<string, string>  $progress
     */
    private function arrangeLive(array $identity, array $progress): NativeEvidencePublicationFence
    {
        $init = ['identity' => $identity, 'progress' => ['wal_flush_lsn' => '0/1', 'xmax' => '1'], 'in_flight' => ['sessions' => 0, 'prepared' => 0]];
        $intent = ['identity' => $identity, 'progress' => $progress, 'in_flight' => ['sessions' => 0, 'prepared' => 0]];
        $hashes = $this->writeRun($this->evidenceDirectory, self::FIXTURE_RUN, $this->standardRecords($this->storeId, [
            'initDatabase' => $init,
            'intentDatabase' => $intent,
        ]));

        return new NativeEvidencePublicationFence(self::FIXTURE_RUN, $hashes[4]);
    }

    /**
     * @param  array<string, mixed>  $changes
     * @return array<string, mixed>
     */
    private function withIdentity(array $changes): array
    {
        $database = $this->fixtureDatabase('0/2000000', '710');
        $database['identity'] = $changes + $database['identity'];

        return $database;
    }

    /** @return array{string, string, string, string} */
    private function callArguments(): array
    {
        return [NativeEvidenceContract::FRESH_ISSUER, NativeEvidenceContract::FRESH_PROFILE, self::FIXTURE_PURPOSE, self::FIXTURE_KEY_ID];
    }

    private function publish(?NativeEvidencePublicationFence $fence): void
    {
        app(NativeSigningTrustRegistry::class)->publishTrustedKey(
            ...$this->callArguments(),
            ...[str_repeat("\x05", 32), $fence],
        );
    }

    private function floorPath(): string
    {
        return $this->witnessDirectory.'/'.hash('sha256', "signing-trust-state\0".NativeEvidenceContract::FRESH_ISSUER."\0".NativeEvidenceContract::FRESH_PROFILE."\0".self::FIXTURE_PURPOSE).'.floor';
    }

    private function assertNothingWitnessedOrStored(): void
    {
        self::assertFileDoesNotExist($this->floorPath());
        self::assertSame([], glob($this->witnessDirectory.'/*.floor') ?: []);
        self::assertSame(0, DB::table('native_game_signing_trust_profiles')->count());
        self::assertSame(0, DB::table('native_game_signing_trust_key_versions')->count());
    }

    private function requirePgsql(): void
    {
        if (DB::connection()->getDriverName() !== 'pgsql') {
            self::markTestSkipped('Requires the pgsql suite.');
        }
    }
}
