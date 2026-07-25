<?php

namespace App\Localization;

final readonly class LocalizedPublicUrls
{
    /** @param array<string, string> $alternates */
    public function __construct(
        public ?string $canonical,
        public array $alternates,
    ) {}
}
