<?php

namespace App\Wiki\Application\Search;

use Illuminate\Support\Carbon;

final readonly class WikiSearchResult
{
    public function __construct(
        public int $articleId,
        public string $title,
        public string $slug,
        public string $summary,
        public Carbon $publishedAt,
    ) {}
}
