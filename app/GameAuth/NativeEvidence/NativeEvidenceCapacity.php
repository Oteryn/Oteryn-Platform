<?php

namespace App\GameAuth\NativeEvidence;

use Closure;

final class NativeEvidenceCapacity
{
    /** @template T @param Closure(): T $callback @return T */
    public function run(Closure $callback): mixed
    {
        $directory = config('game-auth.native_evidence.high_water_directory');
        $realDirectory = is_string($directory) ? realpath($directory) : false;
        if (! is_string($realDirectory) || ! is_dir($realDirectory) || ! is_writable($realDirectory)) {
            throw new NativeEvidenceUnavailable('Native evidence capacity state is unavailable.');
        }

        for ($slot = 0; $slot < 2; $slot++) {
            $handle = @fopen($realDirectory.DIRECTORY_SEPARATOR."pipeline-{$slot}.lock", 'c+b');
            if (! is_resource($handle)) {
                continue;
            }
            if (!@flock($handle, LOCK_EX | LOCK_NB)) {
                fclose($handle);
                continue;
            }

            try {
                return $callback();
            } finally {
                @flock($handle, LOCK_UN);
                fclose($handle);
            }
        }

        throw new NativeEvidenceUnavailable('Native evidence pipeline capacity is exhausted.');
    }
}
