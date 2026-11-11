<?php

namespace App\Wiki\ViewModels\Public;

final readonly class WikiCategoryPageViewModel
{
    /**
     * @param  list<WikiCategoryCard>  $children
     * @param  list<WikiArticleCard>  $articles
     * @param  list<WikiBreadcrumb>  $breadcrumbs
     */
    public function __construct(
        public int $id,
        public string $name,
        public string $slug,
        public ?string $description,
        public array $children,
        public array $articles,
        public array $breadcrumbs,
    ) {}
}
