<?php

namespace App\PublicGameData;

use App\CanaryIntegration\CanaryRuntimeRedisReader;
use Throwable;

final readonly class CanaryChannelRuntimeService
{
    public function __construct(private CanaryRuntimeRedisReader $reader) {}

    /**
     * @param  list<int>  $channelIds
     */
    public function snapshot(array $channelIds): CanaryRuntimeSnapshot
    {
        if (! PublicGameDataSource::isAvailable()) {
            return CanaryRuntimeSnapshot::unavailable();
        }

        $statuses = [];

        try {
            foreach ($channelIds as $channelId) {
                $statuses[$channelId] = $this->reader->read($channelId);
            }
        } catch (Throwable) {
            return CanaryRuntimeSnapshot::unavailable();
        }

        return CanaryRuntimeSnapshot::available($statuses);
    }
}
