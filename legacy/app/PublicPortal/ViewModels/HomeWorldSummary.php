<?php

namespace App\PublicPortal\ViewModels;

use App\LiveOps\WorldStatus\PublicWorldStatus;
use App\PublicPortal\PublicContentState;

final readonly class HomeWorldSummary
{
    /**
     * @param  list<HomeWorldChannel>  $channels
     */
    public function __construct(
        public PublicContentState $state,
        public array $channels,
        public ?int $playersOnline,
        /** @var list<PublicWorldStatus>|null */
        public ?array $nativeWorlds = null,
    ) {}
}
