<?php

namespace App\Wiki\ViewModels\Public;

final readonly class WikiHomeViewModel
{
    /**
     * @param  list<WikiCategoryCard>  $categories
     * @param  list<WikiArticleCard>  $featuredArticles
     * @param  list<WikiArticleCard>  $recentArticles
     */
    public function __construct(
        public array $categories,
        public array $featuredArticles,
        public array $recentArticles,
    ) {}

    public function isEmpty(): bool
    {
        return $this->categories === []
            && $this->featuredArticles === []
            && $this->recentArticles === [];
    }
}
