<?php

namespace App\GameAuth\Worlds;

final readonly class GameWorldProtocolPolicy
{
    /**
     * @param  list<GameWorldProtocolCandidateRoute>  $candidates
     */
    public function __construct(
        public int $revision,
        public int $channelId,
        public array $candidates,
    ) {}
}
