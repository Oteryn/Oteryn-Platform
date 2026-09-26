<?php

namespace App\Console\Commands;

use App\GameAuth\Worlds\NativeTopologyRegistry;
use Illuminate\Console\Command;
use Throwable;

final class IssueNativeTopology extends Command
{
    protected $signature = 'game-auth:native-topology:issue
        {--world-row-id= : Explicitly provisioned local World row, never a canonical WorldId}
        {--channel-key= : Explicit stable logical Channel key within that World}';

    protected $description = 'Issue and read back one Registry-owned disposable preproduction WorldId + ChannelId.';

    public function handle(NativeTopologyRegistry $registry): int
    {
        $rowId = $this->option('world-row-id');
        $channelKey = $this->option('channel-key');
        if (! is_string($rowId) || preg_match('/\A[1-9][0-9]*\z/', $rowId) !== 1
            || filter_var($rowId, FILTER_VALIDATE_INT) === false || ! is_string($channelKey)) {
            $this->components->error('Supply a positive local --world-row-id and an explicit --channel-key.');

            return self::FAILURE;
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
}
