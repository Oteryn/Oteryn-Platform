<?php

namespace App\Events\ViewModels;

use App\PublicPortal\PublicContentState;
use Carbon\CarbonImmutable;

final readonly class UpcomingEventSummary
{
    /**
     * @var array{id: int, title: string, slug: string, summary: string, starts_at: CarbonImmutable, ends_at: CarbonImmutable, status: string, featured: bool}|null
     */
    public ?array $event;

    /**
     * @param  array{id: int, title: string, slug: string, summary: string, starts_at: CarbonImmutable, ends_at: CarbonImmutable, status: string, featured: bool}|null  $event
     */
    public function __construct(
        public PublicContentState $state,
        ?array $event,
    ) {
        $this->event = $event;
    }
}
