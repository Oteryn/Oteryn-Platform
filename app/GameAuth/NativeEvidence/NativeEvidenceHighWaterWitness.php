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

    private function resolvedDirectory(string $namespaceHash): string
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

        $this->assertStoreProvenance($realDirectory);

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
        if (Schema::hasTable('native_game_signing_trust_profiles')
            && DB::table('native_game_signing_trust_profiles')->exists()) {
            return true;
        }

        return Schema::hasColumn('identities', 'native_security_generation')
            && DB::table('identities')->where('native_security_generation', '>', 1)->exists();
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
