<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\NativeEvidence\NativeEvidenceContract;
use App\GameAuth\NativeEvidence\NativeEvidenceHighWaterWitness;
use App\GameAuth\NativeEvidence\NativeEvidenceNamespace;
use App\GameAuth\NativeEvidence\NativeSigningTrustRegistry;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class NativeSigningTrustObservationCommandTest extends TestCase
{
    use RefreshDatabase;

    private const COMMAND = 'game-auth:native-evidence:observe-trust';

    private const PURPOSE = 'fresh_admission';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = storage_path('framework/testing/native-trust-observation-'.bin2hex(random_bytes(6)));
        mkdir($this->directory, 0700, true);
        config([
            'game-auth.native_evidence.high_water_directory' => $this->directory,
            'game-auth.native_evidence.fresh_key_purpose' => self::PURPOSE,
            'game-auth.native_evidence.first_publication_fence' => 'off',
            'game-auth.native_admission.signing_key_id' => null,
        ]);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->directory.'/*') ?: [] as $path) {
            @unlink($path);
        }
        @rmdir($this->directory);

        parent::tearDown();
    }

    public function test_an_empty_directory_is_observed_without_creating_any_file_lock_or_row(): void
    {
        self::assertSame(0, $this->observe(['--key-id' => 'kid-1']));
        $output = $this->commandOutput();

        self::assertSame('observed', $output['result']);
        self::assertFalse($output['fence_required']);
        self::assertSame(['marker' => null, 'row' => null, 'binding' => 'none'], $output['store']);
        self::assertSame(['value' => null, 'temp_entries' => 0], $output['floor']);
        self::assertSame(['present' => false], $output['profile']);
        self::assertSame(
            ['requested' => 'kid-1', 'configured_key_id' => null, 'in_history' => false, 'trusted' => false],
            $output['key'],
        );
        self::assertSame(
            ['issuer' => NativeEvidenceContract::FRESH_ISSUER, 'profile' => NativeEvidenceContract::FRESH_PROFILE, 'key_purpose' => self::PURPOSE],
            $output['scope'],
        );
        self::assertSame([], glob($this->directory.'/*') ?: []);
        self::assertSame(0, DB::table('native_game_evidence_witness_stores')->count());
    }

    #[DataProvider('bindings')]
    public function test_each_binding_value_is_reported_without_repairing_it(string $binding): void
    {
        $one = str_repeat('1', 64);
        $two = str_repeat('2', 64);
        $marker = in_array($binding, ['match', 'mismatch', 'marker_only'], true) ? $one : null;
        $row = match ($binding) {
            'match' => $one,
            'mismatch', 'row_only' => $two,
            default => null,
        };
        if ($marker !== null) {
            file_put_contents($this->directory.'/witness-store.id', $marker."\n");
        }
        if ($row !== null) {
            DB::table('native_game_evidence_witness_stores')->insert(['id' => 1, 'store_id' => $row, 'created_at' => now()]);
        }
        $files = glob($this->directory.'/*') ?: [];

        self::assertSame(0, $this->observe(['--key-id' => 'kid-1']));

        self::assertSame(['marker' => $marker, 'row' => $row, 'binding' => $binding], $this->commandOutput()['store']);
        self::assertSame($files, glob($this->directory.'/*') ?: []);
        self::assertSame($row === null ? 0 : 1, DB::table('native_game_evidence_witness_stores')->count());
    }

    /** @return array<string, array{string}> */
    public static function bindings(): array
    {
        return [
            'match' => ['match'],
            'mismatch' => ['mismatch'],
            'marker_only' => ['marker_only'],
            'row_only' => ['row_only'],
            'none' => ['none'],
        ];
    }

    public function test_profile_key_and_floor_fields_after_a_publication(): void
    {
        app(NativeEvidenceHighWaterWitness::class)->initializeStoreProvenance();
        app(NativeSigningTrustRegistry::class)->publishTrustedKey(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
            'kid-1',
            str_repeat("\x05", 32),
        );
        $before = $this->directorySnapshot();

        self::assertSame(0, $this->observe(['--key-id' => 'kid-1']));
        $output = $this->commandOutput();

        self::assertSame('match', $output['store']['binding']);
        self::assertSame(1, $output['floor']['value']);
        self::assertTrue($output['profile']['present']);
        self::assertSame(1, $output['profile']['profile_version']);
        self::assertSame(1, $output['profile']['issuer_revision']);
        self::assertFalse($output['profile']['revoked']);
        self::assertIsInt($output['profile']['id']);
        self::assertTrue($output['key']['in_history']);
        self::assertTrue($output['key']['trusted']);
        self::assertSame($before, $this->directorySnapshot());

        self::assertSame(0, $this->observe(['--key-id' => 'kid-2']));
        $other = $this->commandOutput()['key'];
        self::assertFalse($other['in_history']);
        self::assertFalse($other['trusted']);
    }

    public function test_fence_required_follows_the_setting(): void
    {
        config(['game-auth.native_evidence.first_publication_fence' => 'required']);

        self::assertSame(0, $this->observe(['--key-id' => 'kid-1']));

        self::assertTrue($this->commandOutput()['fence_required']);
    }

    public function test_witness_busy_while_the_store_lock_is_held(): void
    {
        app(NativeEvidenceHighWaterWitness::class)->initializeStoreProvenance();

        $this->assertBusyWhileHeld($this->directory.'/witness-store.lock');
    }

    public function test_witness_busy_while_the_trust_floor_lock_is_held(): void
    {
        $namespace = NativeEvidenceNamespace::trustState(
            NativeEvidenceContract::FRESH_ISSUER,
            NativeEvidenceContract::FRESH_PROFILE,
            self::PURPOSE,
        );
        file_put_contents($this->directory.'/'.$namespace.'.floor.lock', '');

        $this->assertBusyWhileHeld($this->directory.'/'.$namespace.'.floor.lock');
    }

    public function test_an_unconfigured_key_purpose_gives_scope_unconfigured(): void
    {
        config(['game-auth.native_evidence.fresh_key_purpose' => '']);

        self::assertSame(1, $this->observe(['--key-id' => 'kid-1']));

        self::assertSame('scope_unconfigured', $this->commandOutput()['result']);
    }

    public function test_only_the_fresh_trust_is_supported(): void
    {
        self::assertSame(1, Artisan::call(self::COMMAND, ['--trust' => 'recovery', '--key-id' => 'kid-1']));
        self::assertSame(1, Artisan::call(self::COMMAND, ['--key-id' => 'kid-1']));
    }

    public function test_database_identity_is_null_off_postgres_and_complete_on_postgres(): void
    {
        self::assertSame(0, $this->observe(['--key-id' => 'kid-1']));
        $database = $this->commandOutput()['database'];

        if (DB::connection()->getDriverName() !== 'pgsql') {
            self::assertNull($database);

            return;
        }
        self::assertIsArray($database);
        foreach (['system_identifier', 'timeline_id', 'postmaster_start_time', 'database_oid', 'relfilenodes'] as $field) {
            self::assertArrayHasKey($field, $database['identity']);
        }
        self::assertArrayHasKey('native_game_signing_trust_profiles', $database['identity']['relfilenodes']);
        self::assertMatchesRegularExpression('/\A[0-9A-F]+\/[0-9A-F]+\z/', $database['progress']['wal_flush_lsn']);
        self::assertMatchesRegularExpression('/\A[0-9]+\z/', $database['progress']['xmax']);
        self::assertArrayHasKey('sessions', $database['in_flight']);
        self::assertArrayHasKey('prepared', $database['in_flight']);
    }

    private function assertBusyWhileHeld(string $lockPath): void
    {
        $lock = fopen($lockPath, 'c+b');
        self::assertIsResource($lock);
        self::assertTrue(flock($lock, LOCK_EX | LOCK_NB));

        try {
            self::assertSame(1, $this->observe(['--key-id' => 'kid-1']));
            self::assertSame(['result' => 'witness_busy'], (array) $this->commandOutput());
        } finally {
            flock($lock, LOCK_UN);
            fclose($lock);
        }
    }

    /** @param array<string, string> $options */
    private function observe(array $options): int
    {
        return Artisan::call(self::COMMAND, ['--trust' => 'fresh'] + $options);
    }

    /** @return array{result?: string, store: array{binding: string}, floor: array{value: int|null}, profile: array{present: bool, profile_version: int, issuer_revision: int, revoked: bool, id: mixed}, key: array{in_history: bool, trusted: bool}, database: array{identity: array{relfilenodes: array<string, int>}&array<string, mixed>, progress: array{wal_flush_lsn: string, xmax: string}, in_flight: array<string, int>}|null} */
    private function commandOutput(): array
    {
        /** @var array{result?: string, store: array{binding: string}, floor: array{value: int|null}, profile: array{present: bool, profile_version: int, issuer_revision: int, revoked: bool, id: mixed}, key: array{in_history: bool, trusted: bool}, database: array{identity: array{relfilenodes: array<string, int>}&array<string, mixed>, progress: array{wal_flush_lsn: string, xmax: string}, in_flight: array<string, int>}|null} $decoded */
        $decoded = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);

        return $decoded;
    }

    /** @return array<string, string> */
    private function directorySnapshot(): array
    {
        $files = [];
        foreach (glob($this->directory.'/*') ?: [] as $path) {
            $files[basename($path)] = (string) file_get_contents($path);
        }

        return $files;
    }
}
