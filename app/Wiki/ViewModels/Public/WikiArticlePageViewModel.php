<?php

namespace App\Wiki\ViewModels\Public;

use Illuminate\Support\Carbon;

final readonly class WikiArticlePageViewModel
{
    /**
     * @param  list<WikiCategoryCard>  $categories
     * @param  list<WikiArticleCard>  $relatedArticles
     * @param  list<WikiBreadcrumb>  $breadcrumbs
     */
    public function __construct(
        public int $id,
        public string $title,
        public string $slug,
        public string $summary,
        public string $sourceMarkdown,
        public Carbon $publishedAt,
        public array $categories,
        public array $relatedArticles,
        public array $breadcrumbs,
    ) {}
}
