<?php

namespace App\PublicPortal\Application\Search\Providers;

use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Cms\PublicNewsQuery;
use App\Cms\PublicPageQuery;
use App\PublicPortal\Application\Search\FederatedSearchProvider;
use App\PublicPortal\Application\Search\FederatedSearchQuery;
use App\PublicPortal\Application\Search\SearchHit;
use App\PublicPortal\Application\Search\SearchProviderResult;
use DateTimeInterface;

final readonly class CmsFederatedSearchProvider implements FederatedSearchProvider
{
    public function __construct(
        private PublicNewsQuery $news,
        private PublicPageQuery $pages,
    ) {}

    public function id(): string
    {
        return 'cms';
    }

    public function search(FederatedSearchQuery $query): SearchProviderResult
    {
        $newsEnabled = $query->allowsType('news');
        $pagesEnabled = $query->allowsType('page');
        if ($newsEnabled && $pagesEnabled) {
            [$newsWindow, $pagesWindow] = ProviderSearchSupport::alternatingWindows($query->page, $query->limit);
        } else {
            $single = ['offset' => $query->offset(), 'count' => $query->limit];
            $none = ['offset' => 0, 'count' => 0];
            $newsWindow = $newsEnabled ? $single : $none;
            $pagesWindow = $pagesEnabled ? $single : $none;
        }

        $news = [];
        if ($newsWindow['count'] > 0) {
            $news = array_values($this->news
                ->searchPublished($query->query, $query->locale, $newsWindow['offset'], $newsWindow['count'])
                ->map(fn (NewsPost $post): SearchHit => new SearchHit(
                    sourceModule: 'CMS',
                    sourceType: 'news',
                    stablePublicId: 'news-'.$post->id,
                    sourceRevision: $post->updated_at->toIso8601String(),
                    locale: $query->locale,
                    title: $post->title,
                    snippetPlaintext: ProviderSearchSupport::snippet($post->body),
                    canonicalUrl: route('news.show', ['locale' => $query->locale, 'slug' => $post->slug]),
                    providerRank: 0,
                    publishedOrEffectiveAt: $post->published_at?->toIso8601String(),
                ))
                ->all());
        }

        $pages = [];
        if ($pagesWindow['count'] > 0) {
            $pages = array_values($this->pages
                ->searchPublished($query->query, $query->locale, $pagesWindow['offset'], $pagesWindow['count'])
                ->map(fn (ManagedPage $page): SearchHit => new SearchHit(
                    sourceModule: 'CMS',
                    sourceType: 'page',
                    stablePublicId: 'page-'.$page->id,
                    sourceRevision: $this->pageRevision($page),
                    locale: $query->locale,
                    title: $page->title,
                    snippetPlaintext: ProviderSearchSupport::snippet($page->body),
                    canonicalUrl: $this->pageUrl($page, $query->locale),
                    providerRank: 0,
                    publishedOrEffectiveAt: $page->published_at?->toIso8601String(),
                ))
                ->all());
        }

        return new SearchProviderResult(
            provider: $this->id(),
            available: true,
            hits: ProviderSearchSupport::interleaveWindow($query->page, $query->limit, $news, $pages),
        );
    }

    private function pageUrl(ManagedPage $page, string $locale): string
    {
        $key = EditorialPageKey::fromManagedPageSlug($page->slug);

        return $key === null
            ? route('pages.show', ['locale' => $locale, 'slug' => $page->slug])
            : route($key->publicRouteName(), ['locale' => $locale]);
    }

    private function pageRevision(ManagedPage $page): ?string
    {
        $value = $page->getAttribute('updated_at');

        return $value instanceof DateTimeInterface ? $value->format(DATE_ATOM) : null;
    }
}
