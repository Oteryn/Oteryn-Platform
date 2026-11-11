<?php

namespace App\PublicPortal\Today;

final readonly class TodayPageViewModel
{
    /** @param list<TodayCard> $cards */
    public function __construct(
        public TodayPageState $state,
        public array $cards,
    ) {}
}
