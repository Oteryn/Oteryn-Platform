<?php

namespace App\GameAuth\NativeEvidence;

use Closure;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use stdClass;
use Throwable;

final class NativeEvidenceHighWaterWitness
{
    private const STORE_ROW_ID = 1;

    private const STORE_ID_FILENAME = 'witness-store.id';

    private const STORE_LOCK_FILENAME = 'witness-store.lock';

    public function isConfigured(): bool
    {
        $directory = config('game-auth.native_evidence.high_water_directory');

        return is_string($directory) && $directory !== '';
    }

    public function peek(string $namespaceHash): ?int
    {
        $realDirectory = $this->resolvedDirectory($namespaceHash);

        return $this->readFloor($realDirectory.DIRECTORY_SEPARATOR.$namespaceHash.'.floor');
    }

    /**
     * Execute one source namespace while holding its independent high-water witness lock.
     *
     * @template T
     *
     * @param  Closure(?int, Closure(int): void): T  $callback
     * @return T
     */
    public function withNamespace(string $namespaceHash, Closure $callback): mixed
    {
        $realDirectory = $this->resolvedDirectory($namespaceHash);

        $floorPath = $realDirectory.DIRECTORY_SEPARATOR.$namespaceHash.'.floor';
        $lock = @fopen($floorPath.'.lock', 'c+b');
        $locked = is_resource($lock) ? @flock($lock, LOCK_EX | LOCK_NB) : false;
        if (! is_resource($lock) || $locked !== true) {
            if (is_resource($lock)) {
                fclose($lock);
            }
            throw new NativeEvidenceUnavailable('Native evidence namespace is busy.');
        }

        try {
            $floor = $this->readFloor($floorPath);
            $current = $floor;
            $advance = function (int $revision) use ($floorPath, &$current): void {
                if ($revision < 0 || ($current !== null && $revision < $current)) {
                    throw new NativeEvidenceUnavailable('Native evidence high-water cannot roll back.');
                }
                if ($current === $revision) {
                    return;
                }
                $this->writeFloor($floorPath, $revision);
                $current = $revision;
            };

            return $callback($floor, $advance);
        } finally {
            @flock($lock, LOCK_UN);
            fclose($lock);
        }
    }

    /**
     * Provenance-only init (design note D1). Never calls withNamespace, peek or an advance, never writes a
     * .floor file and never deletes, renames or edits a witness entry. Writes the marker, then the row, and
     * only the missing half of the provenance. Every other state throws with nothing changed.
     */
    public function initializeStoreProvenance(): NativeEvidenceProvenanceResult
    {
        $directory = config('game-auth.native_evidence.high_water_directory');
        if (! function_exists('fsync')) {
            throw new NativeEvidenceUnavailable('Native evidence high-water durability requires PHP fsync support.');
        }
        $realDirectory = is_string($directory) && $directory !== '' ? realpath($directory) : false;
        if (! is_string($realDirectory) || ! is_dir($realDirectory) || ! is_writable($realDirectory)) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness is unavailable.');
        }
        if (! Schema::hasTable('native_game_evidence_witness_stores')) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance schema is unavailable.');
        }

        $lock = @fopen($realDirectory.DIRECTORY_SEPARATOR.self::STORE_LOCK_FILENAME, 'c+b');
        if (! is_resource($lock)) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance lock is unavailable.');
        }

        try {
            if (@flock($lock, LOCK_EX | LOCK_NB) !== true) {
                return new NativeEvidenceProvenanceResult(NativeEvidenceProvenanceResult::BUSY, null, []);
            }

            $entries = $this->classifyEntries($realDirectory);
            $counts = array_map('count', $entries);
            if ($counts['unclassified'] > 0) {
                throw new NativeEvidenceUnavailable('Native evidence witness directory holds an unclassified entry.');
            }
            $floorEntries = $counts['floor'] + $counts['floor-lock'] + $counts['floor-temp'];

            $marker = $this->readStoreId($realDirectory);
            $database = $this->readDatabaseStoreId();

            if ($marker === null && $database !== null) {
                throw new NativeEvidenceUnavailable('Native evidence witness-store row exists without its marker.');
            }
            if ($marker !== null && $database !== null) {
                if (! hash_equals($database, $marker)) {
                    throw new NativeEvidenceUnavailable('Native evidence witness-store provenance does not match retained storage.');
                }

                return new NativeEvidenceProvenanceResult(NativeEvidenceProvenanceResult::UNCHANGED, $marker, $counts);
            }
            if ($floorEntries > 0 || $this->databaseHasNativeHistory()) {
                throw new NativeEvidenceUnavailable(
                    $marker === null
                        ? 'Native evidence witness directory or database holds history without a store identity.'
                        : 'Native evidence witness store has history but no database binding; the init does not re-bind it.'
                );
            }

            if ($marker !== null) {
                $this->insertDatabaseStoreId($marker);

                return new NativeEvidenceProvenanceResult(NativeEvidenceProvenanceResult::COMPLETED, $marker, $counts);
            }

            $storeId = bin2hex(random_bytes(32));
            $this->writeStoreId($realDirectory, $storeId);
            $this->insertDatabaseStoreId($storeId);

            return new NativeEvidenceProvenanceResult(NativeEvidenceProvenanceResult::CREATED, $storeId, $counts);
        } finally {
            @flock($lock, LOCK_UN);
            fclose($lock);
        }
    }

    /**
     * Read-only observation (design note D2). Creates no file, lock file, row or marker and calls neither
     * peek, resolvedDirectory, assertStoreProvenance nor withNamespace. Takes a shared non-blocking lock on
     * each lock file that exists; a busy lock gives result witness_busy. The optional callback runs while
     * the locks are held and its return value is reported as `reads`.
     *
     * @return array{result: string, store?: array<string, mixed>, floor?: array<string, mixed>, reads?: mixed}
     */
    public function observe(string $namespaceHash, ?Closure $reads = null): array
    {
        if (preg_match('/^[0-9a-f]{64}$/', $namespaceHash) !== 1) {
            throw new NativeEvidenceUnavailable('Native evidence namespace identity is invalid.');
        }
        $directory = config('game-auth.native_evidence.high_water_directory');
        $realDirectory = is_string($directory) && $directory !== '' ? realpath($directory) : false;
        if (! is_string($realDirectory) || ! is_dir($realDirectory)) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness is unavailable.');
        }
        if (! Schema::hasTable('native_game_evidence_witness_stores')) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance schema is unavailable.');
        }

        $floorPath = $realDirectory.DIRECTORY_SEPARATOR.$namespaceHash.'.floor';
        $handles = [];
        try {
            foreach ([
                $realDirectory.DIRECTORY_SEPARATOR.self::STORE_LOCK_FILENAME,
                $floorPath.'.lock',
            ] as $lockPath) {
                if (! file_exists($lockPath)) {
                    continue;
                }
                $handle = @fopen($lockPath, 'rb');
                if (! is_resource($handle)) {
                    throw new NativeEvidenceUnavailable('Native evidence witness lock cannot be opened.');
                }
                $handles[] = $handle;
                if (@flock($handle, LOCK_SH | LOCK_NB) !== true) {
                    return ['result' => 'witness_busy'];
                }
            }

            $marker = $this->readStoreId($realDirectory);
            $row = $this->readDatabaseStoreId();
            $binding = match (true) {
                $marker !== null && $row !== null => hash_equals($row, $marker) ? 'match' : 'mismatch',
                $marker !== null => 'marker_only',
                $row !== null => 'row_only',
                default => 'none',
            };

            return [
                'result' => 'observed',
                'store' => ['marker' => $marker, 'row' => $row, 'binding' => $binding],
                'floor' => [
                    'value' => $this->readFloor($floorPath),
                    'temp_entries' => count(glob($floorPath.'.tmp.*') ?: []),
                ],
                'reads' => $reads === null ? null : $reads(),
            ];
        } finally {
            foreach ($handles as $handle) {
                @flock($handle, LOCK_UN);
                fclose($handle);
            }
        }
    }

    /**
     * First-publication entry (design note D8). Holds witness-store.lock shared for the whole call and never
     * reconciles provenance (a marker-only state is refused, not re-bound). Verifies the fence's intent from
     * the evidence directory under both locks, and before the first advance inside the database transaction
     * re-checks database identity, progress, the binding row, the marker and the lock inodes.
     *
     * @template T
     *
     * @param  Closure(?int, Closure(int): void): T  $callback
     * @return T
     */
    public function withNamespaceFenced(string $namespaceHash, NativeEvidencePublicationFence $fence, Closure $callback): mixed
    {
        $realDirectory = $this->resolvedDirectoryWithoutProvenance($namespaceHash);
        if (! Schema::hasTable('native_game_evidence_witness_stores')) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance schema is unavailable.');
        }

        $storeLockPath = $realDirectory.DIRECTORY_SEPARATOR.self::STORE_LOCK_FILENAME;
        $storeLock = file_exists($storeLockPath) ? @fopen($storeLockPath, 'rb') : false;
        if (! is_resource($storeLock)) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance lock is unavailable.');
        }
        if (@flock($storeLock, LOCK_SH | LOCK_NB) !== true) {
            fclose($storeLock);
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance is busy.');
        }

        $floorPath = $realDirectory.DIRECTORY_SEPARATOR.$namespaceHash.'.floor';
        $floorLock = null;
        try {
            $marker = $this->readStoreId($realDirectory);
            $row = $this->readDatabaseStoreId();
            if ($marker === null || $row === null || ! hash_equals($row, $marker)) {
                throw new NativeEvidenceUnavailable('Native evidence witness-store provenance is not bound; a fenced publication does not re-bind it.');
            }

            $floorLock = @fopen($floorPath.'.lock', 'c+b');
            if (! is_resource($floorLock) || @flock($floorLock, LOCK_EX | LOCK_NB) !== true) {
                throw new NativeEvidenceUnavailable('Native evidence namespace is busy.');
            }

            $fence->verify($marker);
            $floor = $this->readFloor($floorPath);
            $current = $floor;
            $advance = function (int $revision) use ($floorPath, &$current, $fence, $marker, $storeLock, $storeLockPath, $floorLock, $realDirectory): void {
                // Re-asserted on every call: a retried DB::transaction attempt has lost the previous attempt's table locks.
                $this->assertFencedBound($fence, $marker, $realDirectory, $storeLock, $storeLockPath, $floorLock, $floorPath.'.lock');
                if ($revision < 0 || ($current !== null && $revision < $current)) {
                    throw new NativeEvidenceUnavailable('Native evidence high-water cannot roll back.');
                }
                if ($current === $revision) {
                    return;
                }
                $this->writeFloor($floorPath, $revision);
                $current = $revision;
            };

            return $callback($floor, $advance);
        } finally {
            if (is_resource($floorLock)) {
                @flock($floorLock, LOCK_UN);
                fclose($floorLock);
            }
            @flock($storeLock, LOCK_UN);
            fclose($storeLock);
        }
    }

    /**
     * @param  resource  $storeLock
     * @param  resource  $floorLock
     */
    private function assertFencedBound(
        NativeEvidencePublicationFence $fence,
        string $storeId,
        string $realDirectory,
        $storeLock,
        string $storeLockPath,
        $floorLock,
        string $floorLockPath,
    ): void {
        if (DB::transactionLevel() < 1) {
            throw new NativeEvidenceUnavailable('The first publication fence must be checked inside the database transaction.');
        }

        $fence->assertDatabaseBound(new NativeEvidenceDatabaseIdentity);

        $row = $this->readDatabaseStoreId();
        if ($row === null || ! hash_equals($storeId, $row)) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store binding differs from the fenced intent.');
        }

        // Last: the marker by path and the lock inodes by path against the held handles.
        $marker = $this->readStoreId($realDirectory);
        if ($marker === null || ! hash_equals($storeId, $marker)) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store marker differs from the fenced intent.');
        }
        foreach ([[$storeLock, $storeLockPath], [$floorLock, $floorLockPath]] as [$handle, $path]) {
            $held = @fstat($handle);
            $named = @stat($path);
            if (! is_array($held) || ! is_array($named)
                || $held['dev'] !== $named['dev'] || $held['ino'] !== $named['ino']) {
                throw new NativeEvidenceUnavailable('Native evidence witness lock was replaced during the fenced publication.');
            }
        }
    }

    /**
     * @return array<string, list<string>> entry names per class (design note D1)
     */
    private function classifyEntries(string $realDirectory): array
    {
        $classes = [
            'marker' => [], 'store-lock' => [], 'floor' => [], 'floor-lock' => [],
            'marker-temp' => [], 'floor-temp' => [], 'unclassified' => [],
        ];
        $names = scandir($realDirectory);
        if ($names === false) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness cannot be listed.');
        }
        foreach ($names as $name) {
            if ($name === '.' || $name === '..') {
                continue;
            }
            $path = $realDirectory.DIRECTORY_SEPARATOR.$name;
            $class = match (true) {
                is_link($path) || ! is_file($path) => 'unclassified',
                $name === self::STORE_ID_FILENAME => 'marker',
                $name === self::STORE_LOCK_FILENAME => 'store-lock',
                preg_match('/\A[0-9a-f]{64}\.floor\z/', $name) === 1 => 'floor',
                preg_match('/\A[0-9a-f]{64}\.floor\.lock\z/', $name) === 1 => 'floor-lock',
                preg_match('/\Awitness-store\.id\.tmp\.[0-9a-f]{16}\z/', $name) === 1 => 'marker-temp',
                preg_match('/\A[0-9a-f]{64}\.floor\.tmp\.[0-9a-f]{16}\z/', $name) === 1 => 'floor-temp',
                default => 'unclassified',
            };
            $classes[$class][] = $name;
        }

        return $classes;
    }

    private function resolvedDirectory(string $namespaceHash): string
    {
        $realDirectory = $this->resolvedDirectoryWithoutProvenance($namespaceHash);

        $this->assertStoreProvenance($realDirectory);

        return $realDirectory;
    }

    private function resolvedDirectoryWithoutProvenance(string $namespaceHash): string
    {
        if (preg_match('/^[0-9a-f]{64}$/', $namespaceHash) !== 1) {
            throw new NativeEvidenceUnavailable('Native evidence namespace identity is invalid.');
        }
        if (! function_exists('fsync')) {
            throw new NativeEvidenceUnavailable('Native evidence high-water durability requires PHP fsync support.');
        }

        $directory = config('game-auth.native_evidence.high_water_directory');
        if (! is_string($directory) || $directory === '') {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness is not configured.');
        }

        $realDirectory = realpath($directory);
        if (! is_string($realDirectory) || ! is_dir($realDirectory) || ! is_writable($realDirectory)) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness is unavailable.');
        }

        return $realDirectory;
    }

    private function assertStoreProvenance(string $realDirectory): void
    {
        if (! Schema::hasTable('native_game_evidence_witness_stores')) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance schema is unavailable.');
        }

        $lockPath = $realDirectory.DIRECTORY_SEPARATOR.self::STORE_LOCK_FILENAME;
        $lock = @fopen($lockPath, 'c+b');
        if (! is_resource($lock)) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance lock is unavailable.');
        }

        try {
            if (@flock($lock, LOCK_SH | LOCK_NB) !== true) {
                throw new NativeEvidenceUnavailable('Native evidence witness-store provenance is busy.');
            }

            $marker = $this->readStoreId($realDirectory);
            $database = $this->readDatabaseStoreId();
            if ($marker !== null && $database !== null) {
                if (! hash_equals($database, $marker)) {
                    throw new NativeEvidenceUnavailable('Native evidence witness-store provenance does not match retained storage.');
                }

                return;
            }

            @flock($lock, LOCK_UN);
            if (@flock($lock, LOCK_EX | LOCK_NB) !== true) {
                throw new NativeEvidenceUnavailable('Native evidence witness-store provenance is busy.');
            }

            $this->reconcileStoreProvenance($realDirectory);
        } finally {
            @flock($lock, LOCK_UN);
            fclose($lock);
        }
    }

    private function reconcileStoreProvenance(string $realDirectory): void
    {
        $marker = $this->readStoreId($realDirectory);
        $database = $this->readDatabaseStoreId();

        if ($marker !== null && $database !== null) {
            if (! hash_equals($database, $marker)) {
                throw new NativeEvidenceUnavailable('Native evidence witness-store provenance does not match retained storage.');
            }

            return;
        }

        if ($database !== null) {
            throw new NativeEvidenceUnavailable('Native evidence retained witness store is missing its durable identity.');
        }

        if ($marker !== null) {
            $this->writeStoreId($realDirectory, $marker);
            $this->insertDatabaseStoreId($marker);

            return;
        }

        if (! $this->hasLegacyFloor($realDirectory) && $this->databaseHasNativeHistory()) {
            throw new NativeEvidenceUnavailable('Native evidence history exists but the retained witness-store identity is missing.');
        }

        $storeId = bin2hex(random_bytes(32));
        $this->writeStoreId($realDirectory, $storeId);
        $this->insertDatabaseStoreId($storeId);
    }

    private function readStoreId(string $realDirectory): ?string
    {
        $path = $realDirectory.DIRECTORY_SEPARATOR.self::STORE_ID_FILENAME;
        if (! file_exists($path)) {
            return null;
        }

        $raw = @file_get_contents($path);
        if (! is_string($raw)) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store identity cannot be read.');
        }

        $storeId = trim($raw);
        if (preg_match('/^[0-9a-f]{64}$/', $storeId) !== 1) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store identity is malformed.');
        }

        return $storeId;
    }

    private function readDatabaseStoreId(): ?string
    {
        if (DB::table('native_game_evidence_witness_stores')->where('id', '<>', self::STORE_ROW_ID)->exists()) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store provenance contains an unexpected authority row.');
        }

        $row = DB::table('native_game_evidence_witness_stores')->where('id', self::STORE_ROW_ID)->first();
        if (! $row instanceof stdClass) {
            return null;
        }
        if (! is_string($row->store_id ?? null) || preg_match('/^[0-9a-f]{64}$/', $row->store_id) !== 1) {
            throw new NativeEvidenceUnavailable('Native evidence witness-store database identity is malformed.');
        }

        return $row->store_id;
    }

    private function insertDatabaseStoreId(string $storeId): void
    {
        DB::table('native_game_evidence_witness_stores')->insert([
            'id' => self::STORE_ROW_ID,
            'store_id' => $storeId,
            'created_at' => now(),
        ]);
    }

    private function hasLegacyFloor(string $realDirectory): bool
    {
        foreach (glob($realDirectory.DIRECTORY_SEPARATOR.'*.floor') ?: [] as $path) {
            if (preg_match('/^[0-9a-f]{64}\.floor$/', basename($path)) === 1) {
                return true;
            }
        }

        return false;
    }

    private function databaseHasNativeHistory(): bool
    {
        if (Schema::hasTable('native_game_evidence_observations')
            && DB::table('native_game_evidence_observations')->exists()) {
            return true;
        }

        return Schema::hasTable('native_game_signing_trust_profiles')
            && DB::table('native_game_signing_trust_profiles')->exists();
    }

    private function readFloor(string $path): ?int
    {
        if (! file_exists($path)) {
            return null;
        }

        $raw = @file_get_contents($path);
        if (! is_string($raw)) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness cannot be read.');
        }

        $value = trim($raw);
        if (preg_match('/^(0|[1-9][0-9]{0,18})$/', $value) !== 1) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness is malformed.');
        }

        $revision = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 0]]);
        if (! is_int($revision)) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness is out of range.');
        }

        return $revision;
    }

    private function writeStoreId(string $realDirectory, string $storeId): void
    {
        $this->writeDurableFile(
            $realDirectory.DIRECTORY_SEPARATOR.self::STORE_ID_FILENAME,
            $storeId."\n",
            'Native evidence witness-store identity',
        );
    }

    private function writeFloor(string $path, int $revision): void
    {
        $this->writeDurableFile($path, (string) $revision."\n", 'Native evidence high-water witness');
    }

    private function writeDurableFile(string $path, string $payload, string $label): void
    {
        $temporary = $path.'.tmp.'.bin2hex(random_bytes(8));
        $handle = @fopen($temporary, 'xb');
        if (! is_resource($handle)) {
            throw new NativeEvidenceUnavailable("{$label} cannot be created.");
        }

        try {
            $written = @fwrite($handle, $payload);
            $flushed = @fflush($handle);
            $synced = @fsync($handle);
            if ($written !== strlen($payload) || $flushed !== true || $synced !== true) {
                throw new NativeEvidenceUnavailable("{$label} cannot be durably persisted.");
            }
        } catch (Throwable $exception) {
            fclose($handle);
            @unlink($temporary);

            throw $exception;
        }
        fclose($handle);

        if (@rename($temporary, $path) !== true) {
            @unlink($temporary);
            throw new NativeEvidenceUnavailable("{$label} cannot be installed.");
        }

        $directory = @fopen(dirname($path), 'r');
        if (! is_resource($directory)) {
            throw new NativeEvidenceUnavailable("{$label} directory cannot be synchronized.");
        }
        try {
            if (@fsync($directory) !== true) {
                throw new NativeEvidenceUnavailable("{$label} directory cannot be synchronized.");
            }
        } finally {
            fclose($directory);
        }
    }
}
