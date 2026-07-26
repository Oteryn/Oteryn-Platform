<?php

namespace Tests\Feature\PublicPortal;

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\EditorialTranslation;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\Wiki\Domain\WikiArticleStatus;
use App\Wiki\Queries\Public\PublicWikiQuery;
use App\Wiki\ViewModels\Public\WikiArticlePageViewModel;
use App\Wiki\ViewModels\Public\WikiCategoryPageViewModel;
use App\Wiki\ViewModels\Public\WikiHomeViewModel;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use RuntimeException;
use Tests\TestCase;

final class HomepageNavigationSeoTest extends TestCase
{
    use RefreshDatabase;

    public function test_homepage_composes_truthful_existing_providers_and_live_quick_links(): void
    {
        SiteAnnouncement::factory()->create(['title' => 'Planned realm maintenance']);
        $event = Event::factory()->create(['status' => Event::STATUS_SCHEDULED]);
        EventTranslation::factory()->create([
            'event_id' => $event->id,
            'locale' => 'en',
            'title' => 'Summer tournament',
            'slug' => 'summer-tournament',
        ]);

        $this->get('/en')
            ->assertOk()
            ->assertSee('data-content-state="AVAILABLE"', false)
            ->assertSeeText('Planned realm maintenance')
            ->assertSeeText('Summer tournament')
            ->assertSee(url('/en/download'), false)
            ->assertSee(url('/en/guilds'), false)
            ->assertSee(url('/en/wiki'), false)
            ->assertSee(url('/en/events'), false)
            ->assertSee(url('/en/support'), false);
    }

    public function test_shared_metadata_is_escaped_localized_and_bounded(): void
    {
        NewsPost::query()->create([
            'slug' => 'safe-news',
            'title' => '<script>alert(1)</script> Realm news',
            'body' => '<b>Published update</b> for every player.',
            'published_at' => now()->subMinute(),
        ]);

        $this->get('/en/news/safe-news')
            ->assertOk()
            ->assertSee('<title>alert(1) Realm news · Oteryn Platform</title>', false)
            ->assertDontSee('<script>', false)
            ->assertSee('<meta name="description" content="Published update for every player.">', false)
            ->assertSee('<meta name="robots" content="index,follow">', false)
            ->assertSee('<meta property="og:type" content="article">', false)
            ->assertSee('<meta property="og:url" content="'.url('/en/news/safe-news').'">', false)
            ->assertSee('hreflang="en"', false)
            ->assertDontSee('hreflang="pl"', false);
    }

    public function test_non_indexable_surfaces_emit_noindex_and_robots_is_not_authorization(): void
    {
        $this->get('/login')
            ->assertOk()
            ->assertSee('<meta name="robots" content="noindex,nofollow,noarchive">', false);

        $this->get('/design/home-v2')
            ->assertOk()
            ->assertSee('<meta name="robots" content="noindex,nofollow">', false);

        $this->get('/en/wiki/search?q=account')
            ->assertOk()
            ->assertSee('<meta name="robots" content="noindex,follow">', false);

        $this->get('/admin')->assertRedirect('/login');
    }

    public function test_robots_points_to_fail_closed_sitemap_and_excludes_private_surfaces(): void
    {
        $this->get('/robots.txt')
            ->assertOk()
            ->assertHeader('Content-Type', 'text/plain; charset=UTF-8')
            ->assertSee('Disallow: /admin')
            ->assertSee('Disallow: /account')
            ->assertSee('Disallow: /en/wiki/search')
            ->assertSee('Sitemap: '.url('/sitemap.xml'));
    }

    public function test_sitemap_contains_only_effectively_published_localized_content(): void
    {
        $published = NewsPost::query()->create([
            'slug' => 'published-news',
            'title' => 'Published news',
            'body' => 'Published body',
            'published_at' => now()->subMinute(),
        ]);
        NewsPost::query()->create([
            'slug' => 'future-news',
            'title' => 'Future news',
            'body' => 'Future body',
            'published_at' => now()->addHour(),
        ]);
        EditorialTranslation::query()->create([
            'content_type' => EditorialContentType::NewsPost->value,
            'content_id' => $published->id,
            'locale' => 'pl',
            'title' => 'Opublikowane wiadomości',
            'body' => 'Opublikowana treść',
            'source_updated_at' => $published->updated_at,
            'published_at' => now()->subMinute(),
        ]);

        $page = ManagedPage::query()->create([
            'slug' => 'lore',
            'title' => 'Lore',
            'body' => 'Published lore',
            'published_at' => now()->subMinute(),
        ]);
        EditorialTranslation::query()->create([
            'content_type' => EditorialContentType::ManagedPage->value,
            'content_id' => $page->id,
            'locale' => 'pl',
            'title' => 'Historia',
            'body' => 'Opublikowana historia',
            'source_updated_at' => $page->updated_at,
            'published_at' => now()->subMinute(),
        ]);

        $event = Event::factory()->create(['status' => Event::STATUS_SCHEDULED]);
        EventTranslation::factory()->create(['event_id' => $event->id, 'locale' => 'en', 'slug' => 'public-event']);
        EventTranslation::factory()->create(['event_id' => $event->id, 'locale' => 'pl', 'slug' => 'publiczne-wydarzenie']);
        $draftEvent = Event::factory()->create(['status' => Event::STATUS_DRAFT]);
        EventTranslation::factory()->create(['event_id' => $draftEvent->id, 'locale' => 'en', 'slug' => 'draft-event']);

        $articleId = DB::table('wiki_articles')->insertGetId([
            'content_type' => 'guide',
            'status' => WikiArticleStatus::PUBLISHED->value,
            'is_featured' => false,
            'sort_order' => 0,
            'published_at' => now()->subMinute(),
            'lock_version' => 1,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
        DB::table('wiki_article_translations')->insert([
            'article_id' => $articleId,
            'locale' => 'en',
            'title' => 'Public guide',
            'slug' => 'public-guide',
            'summary' => 'Guide summary',
            'source_markdown' => 'Guide body',
            'created_at' => now(),
            'updated_at' => now(),
        ]);

        $this->get('/sitemap.xml')
            ->assertOk()
            ->assertHeader('Content-Type', 'application/xml; charset=UTF-8')
            ->assertSee(url('/en'), false)
            ->assertSee(url('/pl'), false)
            ->assertSee(url('/en/news/published-news'), false)
            ->assertSee(url('/pl/news/published-news'), false)
            ->assertSee(url('/en/pages/lore'), false)
            ->assertSee(url('/pl/pages/lore'), false)
            ->assertSee(url('/en/events/public-event'), false)
            ->assertSee(url('/pl/events/publiczne-wydarzenie'), false)
            ->assertSee(url('/en/wiki/public-guide'), false)
            ->assertDontSee('future-news')
            ->assertDontSee('draft-event')
            ->assertDontSee('/pl/wiki/public-guide')
            ->assertDontSee('/admin')
            ->assertDontSee('/search')
            ->assertDontSee('/preview');
    }

    public function test_sitemap_dependency_failure_returns_unavailable_without_partial_urls(): void
    {
        $wiki = new class implements PublicWikiQuery
        {
            public function home(string $locale): WikiHomeViewModel
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function category(string $locale, string $slug): ?WikiCategoryPageViewModel
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function article(string $locale, string $slug): ?WikiArticlePageViewModel
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function equivalentArticleSlug(int $articleId, string $locale): ?string
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function equivalentCategorySlug(int $categoryId, string $locale): ?string
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function publishedArticleId(string $locale, string $slug): ?int
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function visibleCategoryId(string $locale, string $slug): ?int
            {
                throw new RuntimeException('dependency unavailable');
            }

            public function sitemapSlugs(string $locale): array
            {
                throw new RuntimeException('dependency unavailable');
            }
        };
        $this->app->instance(PublicWikiQuery::class, $wiki);

        $response = $this->get('/sitemap.xml')
            ->assertStatus(503)
            ->assertSeeText('Sitemap temporarily unavailable.')
            ->assertDontSee('<urlset', false);

        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
    }
}
