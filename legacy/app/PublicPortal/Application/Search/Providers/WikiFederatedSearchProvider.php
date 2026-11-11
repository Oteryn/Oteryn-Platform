<?php

namespace App\PublicPortal\Application\Search\Providers;

use App\PublicPortal\Application\Search\FederatedSearchProvider;
use App\PublicPortal\Application\Search\FederatedSearchQuery;
use App\PublicPortal\Application\Search\SearchHit;
use App\PublicPortal\Application\Search\SearchProviderResult;
use App\Wiki\Application\Search\WikiSearch;
use App\Wiki\Application\Search\WikiSearchResult;

final readonly class WikiFederatedSearchProvider implements FederatedSearchProvider
{
    public function __construct(private WikiSearch $wiki) {}

    public function id(): string
    {
        return 'wiki';
    }

    public function search(FederatedSearchQuery $query): SearchProviderResult
    {
        if (! $query->allowsType('wiki_article')) {
            return new SearchProviderResult($this->id(), true, []);
        }

        $page = $this->wiki->search($query->locale, $query->query, $query->page, $query->limit);
        $hits = [];
        foreach ($page->items as $index => $result) {
            $hits[] = $this->hit($result, $query->locale, $index + 1);
        }

        return new SearchProviderResult($this->id(), true, $hits);
    }

    private function hit(WikiSearchResult $result, string $locale, int $rank): SearchHit
    {
        return new SearchHit(
            sourceModule: 'Wiki',
            sourceType: 'wiki_article',
            stablePublicId: 'wiki-article-'.$result->articleId,
            sourceRevision: null,
            locale: $locale,
            title: $result->title,
            snippetPlaintext: ProviderSearchSupport::snippet($result->summary),
            canonicalUrl: route('wiki.article', ['locale' => $locale, 'slug' => $result->slug]),
            providerRank: $rank,
            publishedOrEffectiveAt: $result->publishedAt->toIso8601String(),
        );
    }
}
