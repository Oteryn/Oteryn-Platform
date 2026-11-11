<?php

namespace App\Wiki\ViewModels\Public;

final readonly class WikiCategoryCard
{
    public function __construct(
        public int $id,
        public string $name,
        public string $slug,
        public ?string $description,
        public int $articleCount,
    ) {}
}
