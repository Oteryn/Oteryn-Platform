<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Connection;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use LogicException;

final class NativeTopologyRegistry
{
    public function issueForPreproduction(int $localWorldRowId, string $channelKey): NativeTopologyReceipt
    {
        $this->validateSelector($localWorldRowId, $channelKey);
        $connection = $this->isolatedConnection();

        $connection->transaction(function () use ($connection, $localWorldRowId, $channelKey): void {
            // The existing World row serializes even the first absent Channel.
            // All issuer paths take World -> Channel in this order.
            $world = $connection->table('game_worlds')->where('id', $localWorldRowId)->lockForUpdate()->first();
            if ($world === null) {
                throw new LogicException('The explicitly provisioned World does not exist.');
            }

            $worldId = $world->world_id;
            if ($worldId === null) {
                $worldId = (string) Str::uuid7();
                if (! NativeTopologyReceipt::isCanonicalId($worldId)) {
                    throw new LogicException('The UUID producer did not issue a canonical WorldId.');
                }
                $changed = $connection->table('game_worlds')->where('id', $localWorldRowId)
                    ->whereNull('world_id')->update(['world_id' => $worldId]);
                if ($changed !== 1) {
                    throw new LogicException('Canonical WorldId issuance lost its locked owner.');
                }
            } elseif (! is_string($worldId) || ! NativeTopologyReceipt::isCanonicalId($worldId)) {
                throw new LogicException('The retained WorldId is not canonical.');
            }

            $channel = $connection->table('game_channels')->where('game_world_id', $localWorldRowId)
                ->where('channel_key', $channelKey)->lockForUpdate()->first();
            if ($channel === null) {
                $channelId = (string) Str::uuid7();
                new NativeTopologyReceipt($worldId, $channelId);
                $connection->table('game_channels')->insert([
                    'game_world_id' => $localWorldRowId,
                    'channel_id' => $channelId,
                    'channel_key' => $channelKey,
                    'created_at' => now(),
                    'updated_at' => now(),
                ]);
            } else {
                if (! is_string($channel->channel_id)) {
                    throw new LogicException('The retained ChannelId is not canonical.');
                }
                new NativeTopologyReceipt($worldId, $channel->channel_id);
            }
        });

        // No receipt escapes an outer, uncommitted caller transaction.
        $receipt = $this->readbackForPreproduction($localWorldRowId, $channelKey);
        Log::info('Disposable native topology Registry issuance/readback completed.', [
            'service' => 'oteryn-platform-world-registry',
            'purpose' => 'disposable-preproduction-native-topology',
            'world_id' => $receipt->worldId,
            'channel_id' => $receipt->channelId,
        ]);

        return $receipt;
    }

    public function readbackForPreproduction(int $localWorldRowId, string $channelKey): NativeTopologyReceipt
    {
        $this->validateSelector($localWorldRowId, $channelKey);
        $connection = $this->isolatedConnection();
        $owned = $connection->table('game_channels')->join('game_worlds', 'game_worlds.id', '=', 'game_channels.game_world_id')
            ->where('game_worlds.id', $localWorldRowId)->where('game_channels.channel_key', $channelKey)
            ->first(['game_worlds.world_id', 'game_channels.channel_id']);
        if ($owned === null || ! is_string($owned->world_id) || ! is_string($owned->channel_id)) {
            throw new LogicException('No committed scoped native topology issuance exists.');
        }

        return new NativeTopologyReceipt($owned->world_id, $owned->channel_id);
    }

    private function validateSelector(int $localWorldRowId, string $channelKey): void
    {
        if ($localWorldRowId < 1 || preg_match('/\A[a-z0-9][a-z0-9-]{0,63}\z/', $channelKey) !== 1) {
            throw new LogicException('Select a positive local World row and an explicit lower-case Channel key of at most 64 characters.');
        }
    }

    private function isolatedConnection(): Connection
    {
        if (! app()->environment(['testing', 'preproduction'])) {
            throw new LogicException('Native topology issuance/readback is restricted to disposable testing or preproduction.');
        }

        $connection = DB::connection();
        if ($connection->transactionLevel() !== 0) {
            throw new LogicException('Native topology issuance/readback requires its own committed transaction boundary.');
        }

        $database = $connection->getDatabaseName();
        if ($connection->getDriverName() === 'mysql') {
            if ($database !== 'oteryn_concurrency'
                || ! in_array($connection->getConfig('host'), ['127.0.0.1', '::1', 'localhost'], true)
                || ! in_array($connection->getConfig('unix_socket'), [null, ''], true)
                || ! app()->environment('testing')) {
                throw new LogicException('Native topology requires the registered loopback disposable MariaDB test database.');
            }

            return $connection;
        }
        if ($connection->getDriverName() !== 'sqlite') {
            throw new LogicException('Unsupported disposable native topology database profile.');
        }
        if ($database === ':memory:' && app()->environment('testing')) {
            return $connection;
        }

        // Check before opening the issuer connection: the qualifying runner owns
        // this retained disposable directory. These guards do not prove custody.
        $temporaryRoot = realpath(sys_get_temp_dir());
        $directory = realpath(dirname($database));
        if ($temporaryRoot === false || $directory === false
            || dirname($directory) !== $temporaryRoot
            || preg_match('/\Aoteryn-native-topology-[0-9a-f]+\z/', basename($directory)) !== 1
            || $directory !== dirname($database)
            || basename($database) !== 'oteryn-native-topology.sqlite'
            || is_link($directory) || is_link($database)
            || ! is_file($database) || realpath($database) !== $database) {
            throw new LogicException('Native topology requires a retained regular SQLite fixture inside its controlled disposable directory.');
        }

        return $connection;
    }
}
