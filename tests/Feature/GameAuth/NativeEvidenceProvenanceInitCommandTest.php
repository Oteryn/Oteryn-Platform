<?php

namespace Tests\Feature\GameAuth;

use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Illuminate\Support\Facades\DB;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class NativeEvidenceProvenanceInitCommandTest extends TestCase
{
    use RefreshDatabase;

    private const COMMAND = 'game-auth:native-evidence:init-provenance';

    private string $directory;

    protected function setUp(): void
    {
        parent::setUp();

        $this->directory = storage_path('framework/testing/native-provenance-init-'.bin2hex(random_bytes(6)));
        mkdir($this->directory, 0700, true);
        config(['game-auth.native_evidence.high_water_directory' => $this->directory]);
    }

    protected function tearDown(): void
    {
        foreach (glob($this->directory.'/{,.}*', GLOB_BRACE) ?: [] as $path) {
            if (is_link($path) || is_file($path)) {
                @unlink($path);
            } elseif (is_dir($path) && ! in_array(basename($path), ['.', '..'], true)) {
                @rmdir($path);
            }
        }
        @rmdir($this->directory);

        parent::tearDown();
    }

    public function test_created_on_a_clean_slate_writes_the_marker_then_the_row_and_no_floor(): void
    {
        self::assertSame(0, $this->run());
        $output = $this->commandOutput();

        self::assertSame('created', $output['result']);
        self::assertMatchesRegularExpression('/\A[0-9a-f]{64}\z/', $output['store_id']);
        self::assertSame($output['store_id'], trim((string) file_get_contents($this->directory.'/witness-store.id')));
        self::assertSame($output['store_id'], DB::table('native_game_evidence_witness_stores')->value('store_id'));
        self::assertSame([], glob($this->directory.'/*.floor*') ?: []);
        self::assertSame(1, $output['entries']['store-lock']);
        // The counts describe the directory as classified before the init wrote anything.
        self::assertSame(0, $output['entries']['marker']);
    }

    public function test_completed_after_a_crash_between_the_two_writes_inserts_the_row_from_the_marker(): void
    {
        $storeId = bin2hex(random_bytes(32));
        file_put_contents($this->directory.'/witness-store.id', $storeId."\n");

        self::assertSame(0, $this->run());
        $output = $this->commandOutput();

        self::assertSame('completed', $output['result']);
        self::assertSame($storeId, $output['store_id']);
        self::assertSame($storeId, DB::table('native_game_evidence_witness_stores')->value('store_id'));
    }

    public function test_unchanged_reports_floor_class_counts_and_changes_nothing(): void
    {
        self::assertSame(0, $this->run());
        $storeId = $this->commandOutput()['store_id'];
        $hash = str_repeat('a', 64);
        file_put_contents($this->directory.'/'.$hash.'.floor', "1\n");
        file_put_contents($this->directory.'/'.$hash.'.floor.lock', '');
        file_put_contents($this->directory.'/'.$hash.'.floor.tmp.'.str_repeat('b', 16), '1');
        $before = $this->snapshot();

        self::assertSame(0, $this->run());
        $output = $this->commandOutput();

        self::assertSame('unchanged', $output['result']);
        self::assertSame($storeId, $output['store_id']);
        self::assertSame(1, $output['entries']['floor']);
        self::assertSame(1, $output['entries']['floor-lock']);
        self::assertSame(1, $output['entries']['floor-temp']);
        self::assertSame($before, $this->snapshot());
    }

    public function test_marker_temp_entries_are_reported_and_kept(): void
    {
        $temporary = $this->directory.'/witness-store.id.tmp.'.str_repeat('c', 16);
        file_put_contents($temporary, 'partial');

        self::assertSame(0, $this->run());

        self::assertSame(1, $this->commandOutput()['entries']['marker-temp']);
        self::assertSame('partial', file_get_contents($temporary));
    }

    public function test_busy_changes_nothing(): void
    {
        $lock = fopen($this->directory.'/witness-store.lock', 'c+b');
        self::assertIsResource($lock);
        self::assertTrue(flock($lock, LOCK_SH | LOCK_NB));

        try {
            self::assertSame(1, $this->run());
            self::assertSame('busy', $this->commandOutput()['result']);
        } finally {
            flock($lock, LOCK_UN);
            fclose($lock);
        }

        self::assertFileDoesNotExist($this->directory.'/witness-store.id');
        self::assertSame(0, DB::table('native_game_evidence_witness_stores')->count());
    }

    #[DataProvider('refusals')]
    public function test_every_other_state_refuses_with_nothing_changed(string $case): void
    {
        $this->arrange($case);
        $before = $this->snapshot();
        $floorsBefore = glob($this->directory.'/*.floor') ?: [];

        self::assertSame(1, $this->run());

        $output = Artisan::output();
        self::assertStringContainsString('Native evidence provenance init refused:', $output);
        self::assertStringContainsString('"result":"refused"', $output);
        self::assertSame($before, $this->snapshot());
        self::assertSame($floorsBefore, glob($this->directory.'/*.floor') ?: [], 'The init never writes or removes a floor.');
    }

    public function test_a_symlinked_store_lock_refuses_without_creating_its_target(): void
    {
        $target = $this->directory.'-lock-target';
        symlink($target, $this->directory.'/witness-store.lock');

        try {
            self::assertSame(1, $this->run());
            self::assertFileDoesNotExist($target);
        } finally {
            @unlink($this->directory.'/witness-store.lock');
        }
    }

    /** @return array<string, array{string}> */
    public static function refusals(): array
    {
        return [
            'row without marker' => ['row only'],
            'marker and row differ' => ['mismatch'],
            'marker with a floor entry and no row' => ['marker floor no row'],
            'marker with native history and no row' => ['marker history no row'],
            'floor entry without marker' => ['floor no marker'],
            'floor lock without marker' => ['floor lock no marker'],
            'native history without marker' => ['history no marker'],
            'unclassified file' => ['unclassified file'],
            'subdirectory' => ['subdirectory'],
            'symbolic link' => ['symlink'],
            'malformed marker' => ['malformed marker'],
            'malformed row' => ['malformed row'],
            'more than one row' => ['two rows'],
        ];
    }

    private function arrange(string $case): void
    {
        $one = str_repeat('1', 64);
        $two = str_repeat('2', 64);
        $hash = str_repeat('a', 64);
        switch ($case) {
            case 'row only':
                $this->insertRow(1, $one);
                break;
            case 'mismatch':
                file_put_contents($this->directory.'/witness-store.id', $one."\n");
                $this->insertRow(1, $two);
                break;
            case 'marker floor no row':
                file_put_contents($this->directory.'/witness-store.id', $one."\n");
                file_put_contents($this->directory.'/'.$hash.'.floor', "1\n");
                break;
            case 'marker history no row':
                file_put_contents($this->directory.'/witness-store.id', $one."\n");
                $this->insertProfile();
                break;
            case 'floor no marker':
                file_put_contents($this->directory.'/'.$hash.'.floor', "1\n");
                break;
            case 'floor lock no marker':
                file_put_contents($this->directory.'/'.$hash.'.floor.lock', '');
                break;
            case 'history no marker':
                $this->insertProfile();
                break;
            case 'unclassified file':
                file_put_contents($this->directory.'/notes.txt', 'x');
                break;
            case 'subdirectory':
                mkdir($this->directory.'/nested');
                break;
            case 'symlink':
                symlink($this->directory.'/absent-target', $this->directory.'/witness-store.id.link');
                break;
            case 'malformed marker':
                file_put_contents($this->directory.'/witness-store.id', "not-a-store-id\n");
                break;
            case 'malformed row':
                $this->insertRow(1, 'not-a-store-id');
                break;
            case 'two rows':
                file_put_contents($this->directory.'/witness-store.id', $one."\n");
                $this->insertRow(1, $one);
                $this->insertRow(2, $two);
                break;
        }
    }

    private function insertRow(int $id, string $storeId): void
    {
        DB::table('native_game_evidence_witness_stores')->insert(['id' => $id, 'store_id' => $storeId, 'created_at' => now()]);
    }

    private function insertProfile(): void
    {
        DB::table('native_game_signing_trust_profiles')->insert([
            'issuer' => 'urn:oteryn:platform:game-admission',
            'profile' => 'oteryn-pre-admission-v1',
            'key_purpose' => 'fresh_admission',
            'profile_version' => 1,
            'issuer_revision' => 1,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
    }

    private function run(): int
    {
        return Artisan::call(self::COMMAND);
    }

    /** @return array{result: string, store_id: string, entries: array<string, int>} */
    private function commandOutput(): array
    {
        /** @var array{result: string, store_id: string, entries: array<string, int>} $decoded */
        $decoded = json_decode(Artisan::output(), true, 512, JSON_THROW_ON_ERROR);

        return $decoded;
    }

    /**
     * Directory content (without the lock file the init may create) plus the store rows.
     *
     * @return array{files: array<string, string>, rows: list<array<mixed>>}
     */
    private function snapshot(): array
    {
        $files = [];
        foreach (scandir($this->directory) ?: [] as $name) {
            $path = $this->directory.'/'.$name;
            if (in_array($name, ['.', '..', 'witness-store.lock'], true)) {
                continue;
            }
            $files[$name] = is_link($path) ? 'link' : (is_file($path) ? (string) file_get_contents($path) : 'dir');
        }

        return [
            'files' => $files,
            'rows' => array_values(DB::table('native_game_evidence_witness_stores')->orderBy('id')->get()->map(
                static fn (object $row): array => ['id' => $row->id, 'store_id' => $row->store_id],
            )->all()),
        ];
    }
}
