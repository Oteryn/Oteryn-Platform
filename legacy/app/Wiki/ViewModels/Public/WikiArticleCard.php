<?php

namespace App\Wiki\ViewModels\Public;

use Illuminate\Support\Carbon;

final readonly class WikiArticleCard
{
    public function __construct(
        public int $id,
        public string $title,
        public string $slug,
        public string $summary,
        public Carbon $publishedAt,
    ) {}
}
