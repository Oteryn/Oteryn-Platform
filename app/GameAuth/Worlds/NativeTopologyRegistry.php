<?php

namespace App\GameAuth\Worlds;

use App\Audit\AdminAuditRecorder;
use Illuminate\Database\Connection;
use Illuminate\Support\Facades\Log;
use Illuminate\Support\Str;
use LogicException;

final class NativeTopologyRegistry
{
    public function __construct(private readonly AdminAuditRecorder $audit = new AdminAuditRecorder) {}

    public function issueForPreproduction(int $localWorldRowId, string $channelKey): NativeTopologyReceipt
    {
        $this->validateSelector($localWorldRowId, $channelKey);
        $connection = $this->isolatedConnection();

        $connection->transaction(function () use ($connection, $localWorldRowId, $channelKey): void {
            // The existing World row serializes even the first absent Channel.
            // Acquire it before the first consistent read, so a waiter sees
            // the previous issuer's commit without cross-World Channel gaps.
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
                ->where('channel_key', $channelKey)->first();
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

    /**
     * Publishes the Registry route record of one issued Channel (§7.3, D172 testing/preproduction only).
     * An unchanged endpoint keeps its route_revision; any endpoint change advances the registry route
     * version and so invalidates every outstanding grant for the old revision. `$loginEnabled` is the
     * native login policy of the scope (§7.4 condition 2; U7 for production).
     */
    public function publishRouteForPreproduction(
        int $localWorldRowId,
        string $channelKey,
        string $host,
        int $port,
        string $tlsServerName,
        bool $loginEnabled,
    ): NativeRouteRecord {
        $this->validateSelector($localWorldRowId, $channelKey);
        $connection = $this->isolatedConnection();

        $record = $connection->transaction(function () use ($connection, $localWorldRowId, $channelKey, $host, $port, $tlsServerName, $loginEnabled): NativeRouteRecord {
            $world = $connection->table('game_worlds')->where('id', $localWorldRowId)->lockForUpdate()->first();
            $channel = $connection->table('game_channels')->where('game_world_id', $localWorldRowId)
                ->where('channel_key', $channelKey)->lockForUpdate()->first();
            if ($world === null || $channel === null || ! is_string($world->world_id) || ! is_string($channel->channel_id)) {
                throw new LogicException('Publish a route only for an issued native WorldId and ChannelId.');
            }

            $current = NativeRouteRecords::fromRow($world->world_id, $channel);
            $version = $current === null ? 1 + NativeRouteRecords::storedVersion($channel) : $current->version;
            if ($current !== null && ! $current->sameEndpoint($host, $port, $tlsServerName)) {
                $version++;
            }
            $record = new NativeRouteRecord($world->world_id, $channel->channel_id, $host, $port, $tlsServerName, $version);
            $connection->table('game_channels')->where('id', $channel->id)->update([
                'native_route_host' => $record->host,
                'native_route_port' => $record->port,
                'native_route_tls_server_name' => $record->tlsServerName,
                'native_route_version' => $record->version,
                'native_route_revision' => $record->routeRevision,
                'native_login_enabled' => $loginEnabled,
                'updated_at' => now(),
            ]);

            return $record;
        });

        Log::info('Disposable native route record published.', [
            'service' => 'oteryn-platform-world-registry',
            'world_id' => $record->worldId,
            'channel_id' => $record->channelId,
            'route_revision' => $record->routeRevision,
            'native_login_enabled' => $loginEnabled,
        ]);

        return $record;
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

    /**
     * Preproduction-store issuance (ADR 0044): the same Registry issuance as `issueForPreproduction`,
     * on the allowlisted deployed preproduction store admitted by `PreproductionNativeRouteStore`
     * instead of the disposable store. It writes only the selected World row's WorldId (minted by the
     * Registry when absent) and its one Channel, refuses a store on which another World row holds a
     * WorldId, refuses a retained WorldId other than the configured native World, and records one
     * admin audit event in the same transaction. Idempotent: a re-run with an issued scope writes only
     * the audit event.
     *
     * @return array{receipt: NativeTopologyReceipt, result: string}
     */
    public function issueOnPreproductionStore(int $localWorldRowId, string $channelKey): array
    {
        $this->validateSelector($localWorldRowId, $channelKey);
        [$connection, $configuredWorldId] = (new PreproductionNativeRouteStore)->admit();

        $result = $connection->transaction(function () use ($connection, $localWorldRowId, $channelKey, $configuredWorldId): string {
            $world = $connection->table('game_worlds')->where('id', $localWorldRowId)->lockForUpdate()->first();
            if ($world === null) {
                throw new NativePreproductionStoreRefused(NativePreproductionStoreRefused::WORLD_ROW_MISSING, 'The selected World row does not exist.');
            }
            $this->assertNoForeignWorld($connection, $localWorldRowId);

            $result = 'retained';
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
                $result = 'issued';
            } elseif (! is_string($worldId) || ! NativeTopologyReceipt::isCanonicalId($worldId)) {
                throw new LogicException('The retained WorldId is not canonical.');
            } elseif ($worldId !== $configuredWorldId) {
                throw new NativePreproductionStoreRefused(
                    NativePreproductionStoreRefused::WORLD_MISMATCH,
                    'The World row holds WorldId '.$worldId.', which differs from the configured native World.',
                );
            }

            $channel = $connection->table('game_channels')->where('game_world_id', $localWorldRowId)
                ->where('channel_key', $channelKey)->first();
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
                $result = 'issued';
            } else {
                if (! is_string($channel->channel_id)) {
                    throw new LogicException('The retained ChannelId is not canonical.');
                }
                $channelId = $channel->channel_id;
                new NativeTopologyReceipt($worldId, $channelId);
            }

            $this->audit->record(null, 'game_auth.native_topology.preproduction_store_issue', 'native_world', $worldId, [
                'world_row_id' => $localWorldRowId,
                'channel_id' => $channelId,
                'channel_key' => $channelKey,
                'result' => $result,
            ]);

            return $result;
        });

        $owned = $connection->table('game_channels')->join('game_worlds', 'game_worlds.id', '=', 'game_channels.game_world_id')
            ->where('game_worlds.id', $localWorldRowId)->where('game_channels.channel_key', $channelKey)
            ->first(['game_worlds.world_id', 'game_channels.channel_id']);
        if ($owned === null || ! is_string($owned->world_id) || ! is_string($owned->channel_id)) {
            throw new LogicException('No committed scoped native topology issuance exists.');
        }
        $receipt = new NativeTopologyReceipt($owned->world_id, $owned->channel_id);
        Log::info('Preproduction-store native topology issuance/readback completed.', [
            'service' => 'oteryn-platform-world-registry',
            'purpose' => 'preproduction-store-native-topology',
            'world_row_id' => $localWorldRowId,
            'world_id' => $receipt->worldId,
            'channel_id' => $receipt->channelId,
            'result' => $result,
        ]);

        return ['receipt' => $receipt, 'result' => $result];
    }

    /**
     * Preproduction-store route publication (ADR 0044): publishes exactly one route record, for the
     * issued Channel of the configured native World, on the allowlisted deployed preproduction store.
     * It refuses a World row whose WorldId is not the configured one, an unissued Channel, a store on
     * which another World row holds a WorldId, and a store on which any other Channel holds a route
     * record. The endpoint and version rules are those of `publishRouteForPreproduction`. Idempotent:
     * an unchanged endpoint and login policy write no route column, only the admin audit event that
     * every successful run records in the same transaction. The committed record is read back.
     *
     * @return array{record: NativeRouteRecord, result: string, native_login_enabled: bool}
     */
    public function publishRouteOnPreproductionStore(
        int $localWorldRowId,
        string $channelKey,
        string $host,
        int $port,
        string $tlsServerName,
        bool $loginEnabled,
    ): array {
        $this->validateSelector($localWorldRowId, $channelKey);
        [$connection, $configuredWorldId] = (new PreproductionNativeRouteStore)->admit();

        [$record, $result] = $connection->transaction(function () use ($connection, $localWorldRowId, $channelKey, $host, $port, $tlsServerName, $loginEnabled, $configuredWorldId): array {
            $world = $connection->table('game_worlds')->where('id', $localWorldRowId)->lockForUpdate()->first();
            $channel = $connection->table('game_channels')->where('game_world_id', $localWorldRowId)
                ->where('channel_key', $channelKey)->lockForUpdate()->first();
            if ($world === null) {
                throw new NativePreproductionStoreRefused(NativePreproductionStoreRefused::WORLD_ROW_MISSING, 'The selected World row does not exist.');
            }
            if ($world->world_id !== $configuredWorldId) {
                throw new NativePreproductionStoreRefused(
                    NativePreproductionStoreRefused::WORLD_MISMATCH,
                    'The selected World row does not hold the configured native WorldId.',
                );
            }
            if ($channel === null || ! is_string($channel->channel_id)) {
                throw new NativePreproductionStoreRefused(NativePreproductionStoreRefused::CHANNEL_NOT_ISSUED, 'Publish a route only for an issued native ChannelId.');
            }
            $this->assertNoForeignWorld($connection, $localWorldRowId);
            if ($connection->table('game_channels')->where('id', '!=', $channel->id)->whereNotNull('native_route_revision')->exists()) {
                throw new NativePreproductionStoreRefused(
                    NativePreproductionStoreRefused::FOREIGN_ROUTE,
                    'Another Channel on this store already holds a native route record.',
                );
            }

            $current = NativeRouteRecords::fromRow($configuredWorldId, $channel);
            $version = $current === null ? 1 + NativeRouteRecords::storedVersion($channel) : $current->version;
            if ($current !== null && ! $current->sameEndpoint($host, $port, $tlsServerName)) {
                $version++;
            }
            $record = new NativeRouteRecord($configuredWorldId, $channel->channel_id, $host, $port, $tlsServerName, $version);
            $unchanged = $current !== null && $current->routeRevision === $record->routeRevision
                && (bool) $channel->native_login_enabled === $loginEnabled;
            if (! $unchanged) {
                $changed = $connection->table('game_channels')->where('id', $channel->id)->update([
                    'native_route_host' => $record->host,
                    'native_route_port' => $record->port,
                    'native_route_tls_server_name' => $record->tlsServerName,
                    'native_route_version' => $record->version,
                    'native_route_revision' => $record->routeRevision,
                    'native_login_enabled' => $loginEnabled,
                    'updated_at' => now(),
                ]);
                if ($changed !== 1) {
                    throw new LogicException('Native route publication lost its locked Channel.');
                }
            }
            $result = $unchanged ? 'unchanged' : 'published';
            $this->audit->record(null, 'game_auth.native_route.preproduction_store_publish', 'native_channel', $record->channelId, [
                'world_row_id' => $localWorldRowId,
                'world_id' => $record->worldId,
                'route_version' => $record->version,
                'route_revision' => $record->routeRevision,
                'native_login_enabled' => $loginEnabled,
                'result' => $result,
            ]);

            return [$record, $result];
        });

        $row = $connection->table('game_channels')->join('game_worlds', 'game_worlds.id', '=', 'game_channels.game_world_id')
            ->where('game_worlds.id', $localWorldRowId)->where('game_channels.channel_key', $channelKey)
            ->first(['game_worlds.world_id', 'game_channels.*']);
        $stored = $row === null || ! is_string($row->world_id) ? null : NativeRouteRecords::fromRow($row->world_id, $row);
        if ($row === null || $stored === null || $stored->worldId !== $record->worldId || $stored->channelId !== $record->channelId
            || $stored->routeRevision !== $record->routeRevision || (bool) $row->native_login_enabled !== $loginEnabled) {
            throw new LogicException('The committed native route record does not read back.');
        }

        Log::info('Preproduction-store native route record published.', [
            'service' => 'oteryn-platform-world-registry',
            'purpose' => 'preproduction-store-native-route',
            'world_row_id' => $localWorldRowId,
            'world_id' => $stored->worldId,
            'channel_id' => $stored->channelId,
            'route_revision' => $stored->routeRevision,
            'native_login_enabled' => $loginEnabled,
            'result' => $result,
        ]);

        return ['record' => $stored, 'result' => $result, 'native_login_enabled' => $loginEnabled];
    }

    private function assertNoForeignWorld(Connection $connection, int $localWorldRowId): void
    {
        if ($connection->table('game_worlds')->where('id', '!=', $localWorldRowId)->whereNotNull('world_id')->exists()) {
            throw new NativePreproductionStoreRefused(
                NativePreproductionStoreRefused::FOREIGN_WORLD,
                'Another World row on this store already holds a native WorldId.',
            );
        }
    }

    private function validateSelector(int $localWorldRowId, string $channelKey): void
    {
        if ($localWorldRowId < 1 || preg_match('/\A[a-z0-9][a-z0-9-]{0,63}\z/', $channelKey) !== 1) {
            throw new LogicException('Select a positive local World row and an explicit lower-case Channel key of at most 64 characters.');
        }
    }

    private function isolatedConnection(): Connection
    {
        return (new DisposableNativeStore)->connection();
    }
}
