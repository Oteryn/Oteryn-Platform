<?php

namespace App\Console\Commands;

use App\GameAuth\Worlds\NativePreproductionStoreRefused;
use App\GameAuth\Worlds\NativeTopologyRegistry;
use App\GameAuth\Worlds\PreproductionNativeRouteStore;
use Illuminate\Console\Command;
use Illuminate\Support\Facades\Log;
use Throwable;

/**
 * Default path: the disposable per-run store behind `DisposableNativeStore` (ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2),
 * unchanged. With the explicit preproduction-store switch on (ADR 0044) the command instead runs the strictly
 * fenced preproduction-store mode and refuses with a named reason whenever one of its conditions fails.
 */
final class IssueNativeTopology extends Command
{
    protected $signature = 'game-auth:native-topology:issue
        {--world-row-id= : Explicitly provisioned local World row, never a canonical WorldId}
        {--channel-key= : Explicit stable logical Channel key within that World}';

    protected $description = 'Issue and read back one Registry-owned disposable preproduction WorldId + ChannelId.';

    public function handle(NativeTopologyRegistry $registry, PreproductionNativeRouteStore $store): int
    {
        $rowId = $this->option('world-row-id');
        $channelKey = $this->option('channel-key');
        if (! is_string($rowId) || preg_match('/\A[1-9][0-9]*\z/', $rowId) !== 1
            || filter_var($rowId, FILTER_VALIDATE_INT) === false || ! is_string($channelKey)) {
            $this->components->error('Supply a positive local --world-row-id and an explicit --channel-key.');

            return self::FAILURE;
        }

        try {
            $preproductionStore = $store->requested();
        } catch (NativePreproductionStoreRefused $refused) {
            return $this->refusePreproductionStore($refused);
        }
        if ($preproductionStore) {
            return $this->issueOnPreproductionStore($registry, (int) $rowId, $channelKey);
        }

        try {
            $receipt = $registry->issueForPreproduction((int) $rowId, $channelKey);
            $json = json_encode($receipt->toArray(), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        } catch (Throwable) {
            $this->components->error('Disposable native topology issuance/readback failed.');

            return self::FAILURE;
        }

        $this->line($json);

        return self::SUCCESS;
    }

    private function issueOnPreproductionStore(NativeTopologyRegistry $registry, int $rowId, string $channelKey): int
    {
        try {
            $issued = $registry->issueOnPreproductionStore($rowId, $channelKey);
            $json = json_encode([
                'version' => 1,
                'purpose' => 'preproduction-store-native-topology',
                'issuer' => 'oteryn-platform-world-registry',
                'world_id' => $issued['receipt']->worldId,
                'channel_id' => $issued['receipt']->channelId,
                'result' => $issued['result'],
            ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        } catch (NativePreproductionStoreRefused $refused) {
            return $this->refusePreproductionStore($refused);
        } catch (Throwable $exception) {
            Log::warning('Preproduction-store native topology issuance failed.', ['exception' => class_basename($exception)]);
            $this->components->error('Preproduction-store native topology issuance/readback failed.');

            return self::FAILURE;
        }

        $this->line($json);

        return self::SUCCESS;
    }

    private function refusePreproductionStore(NativePreproductionStoreRefused $refused): int
    {
        Log::warning('Preproduction-store native topology issuance refused.', ['reason' => $refused->reason]);
        $this->components->error('Preproduction-store native topology issuance refused: '.$refused->reason.'. '.$refused->getMessage());

        return self::FAILURE;
    }
}
