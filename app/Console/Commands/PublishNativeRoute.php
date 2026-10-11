<?php

namespace App\Console\Commands;

use App\GameAuth\Worlds\DisposableNativeStore;
use App\GameAuth\Worlds\NativePreproductionStoreRefused;
use App\GameAuth\Worlds\NativeRouteRecords;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use App\GameAuth\Worlds\PreproductionNativeRouteStore;
use Illuminate\Console\Command;
use Illuminate\Support\Facades\Log;
use LogicException;
use Throwable;

/**
 * Default path: the retained per-run SQLite store behind `DisposableNativeStore` (ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2),
 * unchanged. With the explicit preproduction-store switch on (ADR 0044) the command instead publishes exactly one
 * route record, for the configured native World, on the allowlisted deployed preproduction store, and refuses with a
 * named reason whenever one of that mode's conditions fails.
 */
final class PublishNativeRoute extends Command
{
    protected $signature = 'game-auth:native-route:publish
        {--world-row-id= : Explicitly provisioned local World row with an issued WorldId}
        {--channel-key= : Issued logical Channel key within that World}
        {--host= : Native gameplay host}
        {--port= : Native gameplay TCP port}
        {--tls-server-name= : TLS server name the client verifies}
        {--login-enabled= : Native login policy of the scope, true or false}';

    protected $description = 'Publish and read back one disposable preproduction native route record (testing/preproduction only).';

    public function handle(NativeTopologyRegistry $registry, DisposableNativeStore $store, PreproductionNativeRouteStore $preproduction): int
    {
        $rowId = $this->option('world-row-id');
        $channelKey = $this->option('channel-key');
        $host = $this->option('host');
        $port = $this->option('port');
        $tlsServerName = $this->option('tls-server-name');
        $loginEnabled = $this->option('login-enabled');
        if (! is_string($rowId) || preg_match('/\A[1-9][0-9]*\z/', $rowId) !== 1
            || filter_var($rowId, FILTER_VALIDATE_INT) === false
            || ! is_string($channelKey) || ! is_string($host) || ! is_string($tlsServerName)
            || ! is_string($port) || preg_match('/\A[1-9][0-9]{0,4}\z/', $port) !== 1 || (int) $port > 65535
            || ! in_array($loginEnabled, ['true', 'false'], true)) {
            $this->components->error('Supply --world-row-id, --channel-key, --host, --port, --tls-server-name and --login-enabled=<true|false>.');

            return self::FAILURE;
        }

        try {
            $preproductionStore = $preproduction->requested();
        } catch (NativePreproductionStoreRefused $refused) {
            return $this->refusePreproductionStore($refused);
        }
        if ($preproductionStore) {
            return $this->publishOnPreproductionStore($registry, (int) $rowId, $channelKey, $host, (int) $port, $tlsServerName, $loginEnabled === 'true');
        }

        try {
            // Environment fence, disposable-store guard and the retained per-run file, before any write.
            [$connection] = $store->retainedRun();
            $record = $registry->publishRouteForPreproduction(
                (int) $rowId,
                $channelKey,
                $host,
                (int) $port,
                $tlsServerName,
                $loginEnabled === 'true',
            );

            $row = $connection->table('game_channels')->join('game_worlds', 'game_worlds.id', '=', 'game_channels.game_world_id')
                ->where('game_worlds.id', (int) $rowId)->where('game_channels.channel_key', $channelKey)
                ->first(['game_worlds.world_id', 'game_channels.*']);
            $stored = $row === null || ! is_string($row->world_id) ? null : NativeRouteRecords::fromRow($row->world_id, $row);
            if ($stored === null || $stored->worldId !== $record->worldId || $stored->channelId !== $record->channelId
                || $stored->routeRevision !== $record->routeRevision) {
                throw new LogicException('The committed native route record does not read back.');
            }

            $json = json_encode([
                'world_id' => $stored->worldId,
                'channel_id' => $stored->channelId,
                'route_version' => $stored->version,
                'route_revision' => $stored->routeRevision,
                'native_login_enabled' => (bool) $row->native_login_enabled,
            ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        } catch (Throwable) {
            $this->components->error('Disposable native route publication failed.');

            return self::FAILURE;
        }

        $this->line($json);

        return self::SUCCESS;
    }

    private function publishOnPreproductionStore(
        NativeTopologyRegistry $registry,
        int $rowId,
        string $channelKey,
        string $host,
        int $port,
        string $tlsServerName,
        bool $loginEnabled,
    ): int {
        try {
            $published = $registry->publishRouteOnPreproductionStore($rowId, $channelKey, $host, $port, $tlsServerName, $loginEnabled);
            $record = $published['record'];
            $json = json_encode([
                'world_id' => $record->worldId,
                'channel_id' => $record->channelId,
                'route_version' => $record->version,
                'route_revision' => $record->routeRevision,
                'native_login_enabled' => $published['native_login_enabled'],
                'store' => 'preproduction',
                'result' => $published['result'],
            ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        } catch (NativePreproductionStoreRefused $refused) {
            return $this->refusePreproductionStore($refused);
        } catch (Throwable $exception) {
            Log::warning('Preproduction-store native route publication failed.', ['exception' => class_basename($exception)]);
            $this->components->error('Preproduction-store native route publication failed.');

            return self::FAILURE;
        }

        $this->line($json);

        return self::SUCCESS;
    }

    private function refusePreproductionStore(NativePreproductionStoreRefused $refused): int
    {
        Log::warning('Preproduction-store native route publication refused.', ['reason' => $refused->reason]);
        $this->components->error('Preproduction-store native route publication refused: '.$refused->reason.'. '.$refused->getMessage());

        return self::FAILURE;
    }
}
