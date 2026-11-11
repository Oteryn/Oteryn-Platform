<?php

namespace Tests\Feature\PublicPortal;

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Models\NewsPost;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\PublicPortal\Today\TodayCardState;
use App\PublicPortal\Today\TodayPageQuery;
use App\PublicPortal\Today\TodayPageState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class PublicTodayTest extends TestCase
{
    use RefreshDatabase;

    public function test_public_today_composes_real_public_sources_in_deterministic_order_without_fabricating_liveops(): void
    {
        SiteAnnouncement::factory()->create([
            'title' => 'Today planned maintenance',
            'body' => 'A bounded public maintenance notice.',
            'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
            'starts_at' => now()->subMinute(),
            'ends_at' => now()->addHour(),
        ]);
        $event = Event::factory()->create([
            'status' => Event::STATUS_SCHEDULED,
            'starts_at' => now()->addDay(),
            'ends_at' => now()->addDay()->addHour(),
        ]);
        EventTranslation::factory()->create([
            'event_id' => $event->id,
            'locale' => 'en',
            'slug' => 'today-tournament',
            'title' => 'Today tournament',
            'summary' => 'A bounded public event summary.',
        ]);
        NewsPost::query()->create([
            'slug' => 'today-news',
            'title' => 'Today news',
            'body' => '<p>A bounded published update.</p>',
            'published_at' => now()->subMinute(),
        ]);

        $response = $this->get('/en/today')
            ->assertOk()
            ->assertSee('data-today-state="partial"', false)
            ->assertSee('data-today-card="liveops"', false)
            ->assertSee('data-content-state="unavailable"', false)
            ->assertSee('data-today-runtime-evidence="absent"', false)
            ->assertSeeText('Today planned maintenance')
            ->assertSeeText('Today tournament')
            ->assertSeeText('Today news')
            ->assertSee('<link rel="canonical" href="'.url('/en/today').'">', false)
            ->assertSee(url('/pl/today'), false)
            ->assertSee('href="'.url('/en/today').'"', false);

        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));

        $html = (string) $response->getContent();
        $liveOps = strpos($html, 'data-today-card="liveops"');
        $announcements = strpos($html, 'data-today-card="announcements"');
        $events = strpos($html, 'data-today-card="events"');
        $news = strpos($html, 'data-today-card="news"');
        self::assertIsInt($liveOps);
        self::assertIsInt($announcements);
        self::assertIsInt($events);
        self::assertIsInt($news);
        self::assertLessThan($announcements, $liveOps);
        self::assertLessThan($events, $announcements);
        self::assertLessThan($news, $events);
    }

    public function test_healthy_empty_sources_remain_distinct_from_unavailable_liveops(): void
    {
        $this->get('/en/today')
            ->assertOk()
            ->assertSee('data-today-card="liveops"', false)
            ->assertSee('data-today-runtime-evidence="absent"', false)
            ->assertSee('data-today-card="announcements"', false)
            ->assertSee('data-today-card="events"', false)
            ->assertSee('data-today-card="news"', false)
            ->assertSeeText('No active announcements.')
            ->assertSeeText('No upcoming event.')
            ->assertSeeText('No published news.');
    }

    public function test_one_provider_failure_yields_truthful_partial_state_instead_of_empty(): void
    {
        $today = app(TodayPageQuery::class)->get(validationScenario: 'news-outage');

        self::assertSame(TodayPageState::PARTIAL, $today->state);
        $news = collect($today->cards)->firstWhere('kind', 'news');
        self::assertNotNull($news);
        self::assertSame(TodayCardState::UNAVAILABLE, $news->state);
    }

    public function test_acceptance_failure_injection_header_is_ignored_outside_acceptance_environment(): void
    {
        NewsPost::query()->create([
            'slug' => 'real-news',
            'title' => 'Real news survives test header',
            'body' => 'Published body.',
            'published_at' => now()->subMinute(),
        ]);

        $this->withHeader('X-Oteryn-Acceptance-Today-Scenario', 'news-outage')
            ->get('/en/today')
            ->assertOk()
            ->assertSeeText('Real news survives test header');
    }

    public function test_today_is_localized_and_present_in_the_public_sitemap(): void
    {
        $this->get('/pl/today')
            ->assertOk()
            ->assertSeeText('Dzisiaj')
            ->assertSee('<html lang="pl">', false)
            ->assertSee('<link rel="canonical" href="'.url('/pl/today').'">', false);

        $this->get('/sitemap.xml')
            ->assertOk()
            ->assertSee(url('/en/today'), false)
            ->assertSee(url('/pl/today'), false);
    }
}
