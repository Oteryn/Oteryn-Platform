<?php

namespace App\PublicPortal\Today;

use DateTimeInterface;

final readonly class TodayItem
{
    public function __construct(
        public string $publicId,
        public string $title,
        public ?string $summary,
        public ?string $url,
        public ?string $actionLabel,
        public ?DateTimeInterface $effectiveAt,
        public ?string $badge = null,
    ) {}
}
