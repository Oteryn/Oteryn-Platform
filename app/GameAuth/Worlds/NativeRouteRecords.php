<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Query\Builder;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

/**
 * Read side of the Registry route records for native issuance (§7.4 conditions 1 and 2). Only a
 * Channel with a complete record whose stored route_revision equals the one recomputed from its
 * descriptor, and whose native login policy is enabled, is returned: a record altered in storage or
 * half-written routes nowhere.
 */
final class NativeRouteRecords
{
    /** @return list<NativeRouteRecord> login-enabled records of one World, ordered by canonical ChannelId */
    public function forWorld(string $worldId): array
    {
        $records = [];
        foreach ($this->query()->where('game_worlds.world_id', $worldId)->orderBy('game_channels.channel_id')->get() as $row) {
            $record = self::fromRow($worldId, $row);
            if ($record !== null) {
                $records[] = $record;
            }
        }

        return $records;
    }

    public function find(string $worldId, string $channelId): ?NativeRouteRecord
    {
        $row = $this->query()->where('game_worlds.world_id', $worldId)->where('game_channels.channel_id', $channelId)->first();

        return $row === null ? null : self::fromRow($worldId, $row);
    }

    /** The verified record of a game_channels row, or null when it is absent, incomplete or altered. */
    public static function fromRow(string $worldId, object $row): ?NativeRouteRecord
    {
        $value = get_object_vars($row);
        $channelId = $value['channel_id'] ?? null;
        $host = $value['native_route_host'] ?? null;
        $port = self::integer($value['native_route_port'] ?? null);
        $tlsServerName = $value['native_route_tls_server_name'] ?? null;
        $version = self::integer($value['native_route_version'] ?? null);
        $revision = $value['native_route_revision'] ?? null;
        if (! is_string($channelId) || ! is_string($host) || $port === null
            || ! is_string($tlsServerName) || $version === null || ! is_string($revision)) {
            return null;
        }

        try {
            $record = new NativeRouteRecord($worldId, $channelId, $host, $port, $tlsServerName, $version);
        } catch (InvalidArgumentException) {
            return null;
        }

        return hash_equals($record->routeRevision, $revision) ? $record : null;
    }

    /** The stored version, also for a record that no longer verifies; 0 when none was published. */
    public static function storedVersion(object $row): int
    {
        return self::integer(get_object_vars($row)['native_route_version'] ?? null) ?? 0;
    }

    private static function integer(mixed $value): ?int
    {
        if (is_string($value) && preg_match('/\A(0|[1-9][0-9]{0,9})\z/', $value) === 1) {
            $value = (int) $value;
        }

        return is_int($value) ? $value : null;
    }

    private function query(): Builder
    {
        return DB::table('game_channels')
            ->join('game_worlds', 'game_worlds.id', '=', 'game_channels.game_world_id')
            ->where('game_channels.native_login_enabled', true)
            ->select('game_channels.*');
    }
}
