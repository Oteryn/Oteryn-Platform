<?php

namespace App\PublicPortal\Today;

use DateTimeInterface;

final readonly class TodayCard
{
    /** @param list<TodayItem> $items */
    public function __construct(
        public string $kind,
        public string $sourceOwner,
        public string $sourceIdentity,
        public ?string $canonicalSourceUrl,
        public TodayCardState $state,
        public int $priority,
        public DateTimeInterface $evaluatedAt,
        public array $items = [],
        public string $applicability = 'public-guest',
        public int $schemaVersion = 1,
    ) {}
}
