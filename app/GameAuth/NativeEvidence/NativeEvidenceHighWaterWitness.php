<?php

namespace App\GameAuth\NativeEvidence;

use Closure;

final class NativeEvidenceHighWaterWitness
{
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
     * @param  Closure(?int, Closure(int): void): T  $callback
     * @return T
     */
    public function withNamespace(string $namespaceHash, Closure $callback): mixed
    {
        $realDirectory = $this->resolvedDirectory($namespaceHash);

        $floorPath = $realDirectory.DIRECTORY_SEPARATOR.$namespaceHash.'.floor';
        $lock = @fopen($floorPath.'.lock', 'c+b');
        if (! is_resource($lock) || ! @flock($lock, LOCK_EX | LOCK_NB)) {
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

    private function writeFloor(string $path, int $revision): void
    {
        $temporary = $path.'.tmp.'.bin2hex(random_bytes(8));
        $handle = @fopen($temporary, 'xb');
        if (! is_resource($handle)) {
            throw new NativeEvidenceUnavailable('Native evidence high-water witness cannot be created.');
        }

        try {
            $payload = (string) $revision."\n";
            if (fwrite($handle, $payload) !== strlen($payload) || ! fflush($handle)) {
                throw new NativeEvidenceUnavailable('Native evidence high-water witness cannot be persisted.');
            }
            if (function_exists('fsync') && ! fsync($handle)) {
                throw new NativeEvidenceUnavailable('Native evidence high-water witness cannot be synchronized.');
            }
        } catch (\Throwable $exception) {
            fclose($handle);
            @unlink($temporary);

            throw $exception;
        }
        fclose($handle);

        if (! @rename($temporary, $path)) {
            @unlink($temporary);
            throw new NativeEvidenceUnavailable('Native evidence high-water witness cannot be installed.');
        }

        if (function_exists('fsync')) {
            $directory = @fopen(dirname($path), 'r');
            if (! is_resource($directory)) {
                throw new NativeEvidenceUnavailable('Native evidence high-water directory cannot be synchronized.');
            }
            try {
                if (! @fsync($directory)) {
                    throw new NativeEvidenceUnavailable('Native evidence high-water directory cannot be synchronized.');
                }
            } finally {
                fclose($directory);
            }
        }
    }
}
