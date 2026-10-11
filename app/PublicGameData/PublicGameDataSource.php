<?php

namespace App\PublicGameData;

use Illuminate\Database\ConnectionInterface;
use Illuminate\Support\Env;
use Illuminate\Support\Facades\DB;

/**
 * Resolves whether the public game-data pages may read the legacy game connection.
 *
 * With `PUBLIC_GAME_DATA_SOURCE=native` the legacy connection is never opened. When
 * the connection is not configured at all, the pages render their unavailable state
 * instead of failing.
 */
final class PublicGameDataSource
{
    public const CONNECTION = 'canary';

    public const NATIVE = 'native';

    public static function isNative(): bool
    {
        $configured = config('public_game_data.source');
        $source = is_string($configured) ? $configured : Env::get('PUBLIC_GAME_DATA_SOURCE');

        return is_string($source) && strtolower(trim($source)) === self::NATIVE;
    }

    public static function isAvailable(): bool
    {
        return ! self::isNative() && is_array(config('database.connections.'.self::CONNECTION));
    }

    /**
     * @throws PublicGameDataUnavailable when the legacy connection must not be used
     */
    public static function assertAvailable(): void
    {
        if (! self::isAvailable()) {
            throw new PublicGameDataUnavailable('Public game data source is not available.');
        }
    }

    /**
     * @throws PublicGameDataUnavailable when the legacy connection must not be used
     */
    public static function connection(): ConnectionInterface
    {
        self::assertAvailable();

        return DB::connection(self::CONNECTION);
    }
}
