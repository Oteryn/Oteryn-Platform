<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use Closure;

final class CharacterBootstrapIntentHighWater
{
    /**
     * @template T
     *
     * @param  Closure(int, Closure(int): void): T  $callback
     * @return T
     */
    public function withLock(Closure $callback): mixed
    {
        $directory = config('game-auth.character_bootstrap_intent.high_water_directory');
        if (! is_string($directory) || $directory === '' || ! is_dir($directory) || ! is_writable($directory)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap high-water directory is unavailable.');
        }
        $lock = fopen($directory.'/authority.lock', 'c+b');
        if ($lock === false || ! flock($lock, LOCK_EX)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap high-water lock is unavailable.');
        }
        try {
            $path = $directory.'/source-revision';
            $raw = is_file($path) ? trim((string) file_get_contents($path)) : '0';
            if (preg_match('/^(0|[1-9][0-9]{0,18})$/', $raw) !== 1) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap high-water is malformed.');
            }
            $floor = (int) $raw;
            $advance = function (int $revision) use ($path): void {
                $temporary = $path.'.'.bin2hex(random_bytes(8)).'.tmp';
                if (file_put_contents($temporary, (string) $revision."\n", LOCK_EX) === false || ! rename($temporary, $path)) {
                    @unlink($temporary);
                    throw new CharacterBootstrapIntentUnavailable('Character bootstrap high-water could not advance.');
                }
            };

            return $callback($floor, $advance);
        } finally {
            flock($lock, LOCK_UN);
            fclose($lock);
        }
    }
}
