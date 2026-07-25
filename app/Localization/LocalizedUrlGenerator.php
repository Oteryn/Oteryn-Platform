<?php

namespace App\Localization;

use App\Cms\Editorial\EditorialContentType;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Editorial\EditorialTranslationResolver;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use DateTimeInterface;
use Illuminate\Http\Request;
use Illuminate\Routing\Route as LaravelRoute;
use Illuminate\Support\Facades\Route;
use Throwable;

final readonly class LocalizedUrlGenerator
{
    public function __construct(
        private PublicLocale $locales,
        private EditorialTranslationResolver $translations,
    ) {}

    public function forRequest(Request $request): LocalizedPublicUrls
    {
        $route = $request->route();
        if (! $route instanceof LaravelRoute) {
            return new LocalizedPublicUrls(null, []);
        }

        $routeName = $route->getName();
        if (! is_string($routeName) || $routeName === '') {
            return new LocalizedPublicUrls(null, []);
        }

        $canonicalName = match (true) {
            $routeName === 'home' => 'localized.home',
            str_starts_with($routeName, 'legacy.') => substr($routeName, 7),
            default => $routeName,
        };
        if (! Route::has($canonicalName)) {
            return new LocalizedPublicUrls(null, []);
        }

        $parameters = $this->stringKeyed($route->parameters());
        unset($parameters['locale']);

        $query = $this->stringKeyed($request->query());
        unset($query['lang']);

        $currentLocale = app()->getLocale();
        $alternates = [];

        foreach ($this->locales->supported() as $locale) {
            $url = $this->equivalent($canonicalName, $parameters, $currentLocale, $locale);
            if ($url !== null) {
                $alternates[$locale] = $this->appendQuery($url, $query);
            }
        }

        return new LocalizedPublicUrls(
            $alternates[$currentLocale] ?? null,
            $alternates,
        );
    }

    /** @param array<string, mixed> $parameters */
    private function equivalent(string $routeName, array $parameters, string $currentLocale, string $targetLocale): ?string
    {
        try {
            return match (true) {
                $routeName === 'news.show' => $this->newsUrl($parameters, $targetLocale),
                $routeName === 'pages.show' => $this->managedPageUrl($routeName, $parameters, $targetLocale),
                $routeName === 'events.show' => $this->eventUrl($parameters, $currentLocale, $targetLocale),
                $this->isTypedEditorialRoute($routeName) => $this->typedEditorialUrl($routeName, $targetLocale),
                default => route($routeName, [...$parameters, 'locale' => $targetLocale]),
            };
        } catch (Throwable) {
            return null;
        }
    }

    /** @param array<string, mixed> $parameters */
    private function newsUrl(array $parameters, string $targetLocale): ?string
    {
        $slug = $parameters['slug'] ?? null;
        if (! is_string($slug)) {
            return null;
        }

        $post = NewsPost::query()
            ->where('slug', $slug)
            ->whereNotNull('published_at')
            ->where('published_at', '<=', now())
            ->first();
        if ($post === null) {
            return null;
        }

        if ($targetLocale !== 'en' && $this->translations->published(
            EditorialContentType::NewsPost,
            $post->id,
            $post->updated_at,
            $targetLocale,
        ) === null) {
            return null;
        }

        return route('news.show', ['locale' => $targetLocale, 'slug' => $post->slug]);
    }

    /** @param array<string, mixed> $parameters */
    private function managedPageUrl(string $routeName, array $parameters, string $targetLocale): ?string
    {
        $slug = $parameters['slug'] ?? null;
        if (! is_string($slug)) {
            return null;
        }

        $page = ManagedPage::query()
            ->where('slug', $slug)
            ->whereNotNull('published_at')
            ->where('published_at', '<=', now())
            ->first();

        return $page === null
            ? null
            : $this->publishedManagedPageUrl($page, $routeName, ['slug' => $page->slug], $targetLocale);
    }

    /** @param array<string, mixed> $parameters */
    private function eventUrl(array $parameters, string $currentLocale, string $targetLocale): ?string
    {
        $slug = $parameters['slug'] ?? null;
        if (! is_string($slug)) {
            return null;
        }

        $current = EventTranslation::query()
            ->join('events', 'events.id', '=', 'event_translations.event_id')
            ->whereIn('events.status', Event::publicStatuses())
            ->where('event_translations.locale', $currentLocale)
            ->where('event_translations.slug', $slug)
            ->select('event_translations.*')
            ->first();
        if ($current === null) {
            return null;
        }

        $target = EventTranslation::query()
            ->where('event_id', $current->event_id)
            ->where('locale', $targetLocale)
            ->first();

        return $target === null
            ? null
            : route('events.show', ['locale' => $targetLocale, 'slug' => $target->slug]);
    }

    private function typedEditorialUrl(string $routeName, string $targetLocale): ?string
    {
        $key = match ($routeName) {
            'editorial.getting-started' => EditorialPageKey::GettingStarted,
            'editorial.server-information' => EditorialPageKey::ServerInformation,
            'support.index' => EditorialPageKey::Support,
            'support.report-a-bug' => EditorialPageKey::ReportABug,
            'editorial.rules' => EditorialPageKey::Rules,
            'legal.terms' => EditorialPageKey::Terms,
            'legal.privacy' => EditorialPageKey::Privacy,
            'legal.cookies' => EditorialPageKey::Cookies,
            default => null,
        };
        if (! $key instanceof EditorialPageKey) {
            return null;
        }

        $page = ManagedPage::query()
            ->where('slug', $key->managedPageSlug())
            ->whereNotNull('published_at')
            ->where('published_at', '<=', now())
            ->first();

        return $page === null
            ? null
            : $this->publishedManagedPageUrl($page, $routeName, [], $targetLocale);
    }

    /** @param array<string, mixed> $parameters */
    private function publishedManagedPageUrl(ManagedPage $page, string $routeName, array $parameters, string $targetLocale): ?string
    {
        if (! ($page->updated_at instanceof DateTimeInterface)) {
            return null;
        }

        if ($targetLocale !== 'en' && $this->translations->published(
            EditorialContentType::ManagedPage,
            $page->id,
            $page->updated_at,
            $targetLocale,
        ) === null) {
            return null;
        }

        return route($routeName, [...$parameters, 'locale' => $targetLocale]);
    }

    private function isTypedEditorialRoute(string $routeName): bool
    {
        return str_starts_with($routeName, 'editorial.')
            || str_starts_with($routeName, 'support.')
            || str_starts_with($routeName, 'legal.');
    }

    /** @param array<string, mixed> $query */
    private function appendQuery(string $url, array $query): string
    {
        if ($query === []) {
            return $url;
        }

        return $url.(str_contains($url, '?') ? '&' : '?')
            .http_build_query($query, '', '&', PHP_QUERY_RFC3986);
    }

    /**
     * @param  array<array-key, mixed>  $values
     * @return array<string, mixed>
     */
    private function stringKeyed(array $values): array
    {
        $normalized = [];

        foreach ($values as $key => $value) {
            if (is_string($key)) {
                $normalized[$key] = $value;
            }
        }

        return $normalized;
    }
}
