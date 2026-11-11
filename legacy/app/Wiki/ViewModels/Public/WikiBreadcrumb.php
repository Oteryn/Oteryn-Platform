<?php

namespace App\Wiki\ViewModels\Public;

final readonly class WikiBreadcrumb
{
    public function __construct(
        public string $label,
        public ?string $url,
    ) {}
}
