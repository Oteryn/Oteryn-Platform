<?php

namespace App\Wiki\Queries\Public;

use App\Wiki\ViewModels\Public\WikiArticlePageViewModel;
use App\Wiki\ViewModels\Public\WikiCategoryPageViewModel;
use App\Wiki\ViewModels\Public\WikiHomeViewModel;

interface PublicWikiQuery
{
    public function home(string $locale): WikiHomeViewModel;

    public function category(string $locale, string $slug): ?WikiCategoryPageViewModel;

    public function article(string $locale, string $slug): ?WikiArticlePageViewModel;

    public function equivalentArticleSlug(int $articleId, string $locale): ?string;

    public function equivalentCategorySlug(int $categoryId, string $locale): ?string;

    public function publishedArticleId(string $locale, string $slug): ?int;

    public function visibleCategoryId(string $locale, string $slug): ?int;
}
