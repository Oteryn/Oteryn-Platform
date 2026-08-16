<?php

namespace App\PublicPortal\Seo;

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Editorial\EditorialTranslationResolver;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Cms\PublicNewsQuery;
use App\Events\Queries\EventCalendarQuery;
use App\Localization\PublicLocale;
use App\Wiki\Queries\Public\PublicWikiQuery;
use DateTimeInterface;
use Illuminate\Support\Facades\Route;
use Throwable;

final readonly class PublicSitemapQuery
{
    public function __construct(
        private PublicLocale $locales,
        private PublicNewsQuery $news,
        private EditorialTranslationResolver $translations,
        private EventCalendarQuery $events,
        private PublicWikiQuery $wiki,
    ) {}

    public function urls(?DateTimeInterface $readTime = null): ?array
    {
        $readTime ??= now();
        $urls = [];

        try {
            foreach ($this->locales->supported() as $locale) {
                foreach ($this->staticRouteNames() as $routeName) {
                    if (Route::has($routeName)) {
                        $urls[] = route($routeName, ['locale' => $locale]);
                    }
                }

                foreach ($this->news->publishedSlugs($readTime) as $slug) {
                    if ($locale !== 'en') {
                        $post = NewsPost::query()->where('slug', $slug)->first();
                        if ($post === null || $this->translations->published(
                            EditorialContentType::NewsPost,
                            $post->id,
                            $post->updated_at,
                            $locale,
                            $readTime,
                        ) === null) {
                            continue;
                        }
                    }
                    $urls[] = route('news.show', ['locale' => $locale, 'slug' => $slug]);
                }

                foreach (ManagedPage::query()
                    ->whereNotNull('published_at')
                    ->where('published_at', '<=', $readTime)
                    ->whereNotIn('slug', $this->typedManagedPageSlugs())
                    ->orderBy('slug')
                    ->get() as $page) {
                    if ($locale !== 'en' && $this->translations->published(
                        EditorialContentType::ManagedPage,
                        $page->id,
                        $page->updated_at,
                        $locale,
                        $readTime,
                    ) === null) {
                        continue;
                    }
                    $urls[] = route('pages.show', ['locale' => $locale, 'slug' => $page->slug]);
                }

                foreach ($this->events->sitemapSlugs($locale, $readTime) as $slug) {
                    $urls[] = route('events.show', ['locale' => $locale, 'slug' => $slug]);
                }

                foreach ($this->wiki->sitemapSlugs($locale) as $slug) {
                    $urls[] = route('wiki.article', ['locale' => $locale, 'slug' => $slug]);
                }
            }
        } catch (Throwable) {
            return null;
        }

        $urls = array_values(array_unique($urls));
        sort($urls, SORT_STRING);

        return $urls;
    }

    /** @return list<string> */
    private function staticRouteNames(): array
    {
        return [
            'localized.home',
            'today.index',
            'news.index',
            'game.highscores.index',
            'game.guilds.index',
            'game.deaths.index',
            'game.online.index',
            'game.servers.index',
            'events.index',
            'downloads.index',
            'editorial.getting-started',
            'editorial.server-information',
            'support.index',
            'support.report-a-bug',
            'editorial.rules',
            'legal.terms',
            'legal.privacy',
            'legal.cookies',
            'wiki.index',
            'game-catalog.index',
            'game-catalog.items.index',
            'game-catalog.creatures.index',
        ];
    }

    /** @return list<string> */
    private function typedManagedPageSlugs(): array
    {
        return array_map(
            static fn (EditorialPageKey $key): string => $key->managedPageSlug(),
            EditorialPageKey::cases(),
        );
    }
}
