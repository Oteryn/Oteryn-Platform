<?php

namespace App\PublicPortal\Seo;

use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Editorial\EditorialPageQuery;
use App\Cms\Editorial\EditorialPageState;
use App\Cms\PublicNewsQuery;
use App\Cms\PublicPageQuery;
use App\Events\Queries\EventCalendarQuery;
use App\Localization\PublicLocale;
use App\Marketplace\Queries\PublicCharacterAuctionQuery;
use App\Wiki\Queries\Public\PublicWikiQuery;
use OverflowException;

final readonly class PublicSitemapQuery
{
    public function __construct(
        private PublicLocale $locales,
        private PublicNewsQuery $news,
        private PublicPageQuery $pages,
        private EditorialPageQuery $editorialPages,
        private EventCalendarQuery $events,
        private PublicCharacterAuctionQuery $marketplace,
        private PublicWikiQuery $wiki,
    ) {}

    /** @return list<string> */
    public function urls(): array
    {
        $originalLocale = app()->getLocale();
        $urls = [];

        try {
            foreach ($this->locales->supported() as $locale) {
                app()->setLocale($locale);

                foreach ($this->staticRouteNames() as $routeName) {
                    $urls[] = route($routeName, ['locale' => $locale]);
                }

                foreach (EditorialPageKey::cases() as $key) {
                    if ($this->editorialPages->find($key)->state === EditorialPageState::Published) {
                        $urls[] = route($key->publicRouteName(), ['locale' => $locale]);
                    }
                }

                foreach ($this->news->publishedSlugs() as $slug) {
                    $urls[] = route('news.show', ['locale' => $locale, 'slug' => $slug]);
                }

                foreach ($this->pages->publishedSlugs() as $slug) {
                    if (! in_array($slug, EditorialPageKey::managedPageSlugs(), true)) {
                        $urls[] = route('pages.show', ['locale' => $locale, 'slug' => $slug]);
                    }
                }

                foreach ($this->events->calendar($locale) as $bucket) {
                    foreach ($bucket as $event) {
                        $urls[] = route('events.show', ['locale' => $locale, 'slug' => $event['slug']]);
                    }
                }

                if (config('marketplace.enabled')) {
                    foreach ($this->marketplace->sitemapIds() as $auctionId) {
                        $urls[] = route('marketplace.show', ['locale' => $locale, 'auction' => $auctionId]);
                    }
                }

                $wikiSlugs = $this->wiki->sitemapSlugs($locale);
                foreach ($wikiSlugs['categories'] as $slug) {
                    $urls[] = route('wiki.category', ['locale' => $locale, 'slug' => $slug]);
                }
                foreach ($wikiSlugs['articles'] as $slug) {
                    $urls[] = route('wiki.article', ['locale' => $locale, 'slug' => $slug]);
                }
            }
        } finally {
            app()->setLocale($originalLocale);
        }

        $urls = array_values(array_unique($urls));
        sort($urls, SORT_STRING);
        if (count($urls) > 50000) {
            throw new OverflowException('The public sitemap exceeds the single-file URL limit.');
        }

        return $urls;
    }

    /** @return list<string> */
    private function staticRouteNames(): array
    {
        $routeNames = [
            'localized.home',
            'today.index',
            'news.index',
            'game.highscores.index',
            'game.guilds.index',
            'game.online.index',
            'game.servers.index',
            'events.index',
            'downloads.index',
            'wiki.index',
        ];

        if (config('marketplace.enabled')) {
            $routeNames[] = 'marketplace.index';
        }

        return $routeNames;
    }
}
