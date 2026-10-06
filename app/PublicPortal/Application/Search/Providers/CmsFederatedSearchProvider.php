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
        $news = [];
        if ($query->allowsType('news')) {
            $news = array_values($this->news
                ->searchPublished($query->query, $query->page, $query->limit)
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
        if ($query->allowsType('page')) {
            $pages = array_values($this->pages
                ->searchPublished($query->query, $query->page, $query->limit)
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
            hits: ProviderSearchSupport::interleave($query->limit, $news, $pages),
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
