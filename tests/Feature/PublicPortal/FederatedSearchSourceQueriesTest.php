<?php

namespace Tests\Feature\PublicPortal;

use App\Announcements\Models\SiteAnnouncement;
use App\Announcements\Queries\ActiveAnnouncementQuery;
use App\Cms\Editorial\EditorialContentType;
use App\Cms\Models\EditorialTranslation;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Cms\PublicNewsQuery;
use App\Cms\PublicPageQuery;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\Events\Queries\EventCalendarQuery;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\App;
use Tests\TestCase;

final class FederatedSearchSourceQueriesTest extends TestCase
{
    use RefreshDatabase;

    protected function tearDown(): void
    {
        App::setLocale('en');
        parent::tearDown();
    }

    public function test_news_search_uses_the_explicit_locale_not_the_global_one(): void
    {
        App::setLocale('en');
        $post = NewsPost::query()->create([
            'slug' => 'locale-news',
            'title' => 'Source dragon news',
            'body' => 'Source body',
            'published_at' => now()->subMinute(),
        ]);
        $this->translate(EditorialContentType::NewsPost, $post->id, $post->updated_at, 'Polski smok', 'Polska tresc');

        $query = app(PublicNewsQuery::class);

        $polish = $query->searchPublished('smok', 'pl');
        $this->assertCount(1, $polish);
        $this->assertSame('Polski smok', $polish->first()?->title);
        $this->assertCount(0, $query->searchPublished('smok', 'en'));
        $this->assertCount(1, $query->searchPublished('dragon', 'en'));
        $this->assertCount(0, $query->searchPublished('dragon', 'pl'));
    }

    public function test_page_search_selects_translated_title_and_body_for_polish(): void
    {
        App::setLocale('en');
        $page = ManagedPage::query()->create([
            'slug' => 'locale-page',
            'title' => 'Source rules page',
            'body' => 'Source rules body',
            'published_at' => now()->subMinute(),
        ]);
        $this->translate(EditorialContentType::ManagedPage, $page->id, $page->updated_at, 'Polskie zasady', 'Polska tresc zasad');

        $hit = app(PublicPageQuery::class)->searchPublished('zasady', 'pl')->first();

        $this->assertNotNull($hit);
        $this->assertSame('Polskie zasady', $hit->title);
        $this->assertSame('Polska tresc zasad', $hit->body);
        $this->assertCount(1, app(PublicPageQuery::class)->searchPublished('rules', 'en'));
    }

    public function test_cms_search_offset_and_limit_window_the_ordered_results(): void
    {
        foreach (range(1, 4) as $number) {
            NewsPost::query()->create([
                'slug' => 'window-'.$number,
                'title' => 'Window story '.$number,
                'body' => 'body',
                'published_at' => now()->subMinutes($number),
            ]);
        }

        $query = app(PublicNewsQuery::class);
        $all = $query->searchPublished('window story', 'en', 0, 4)->pluck('slug')->all();
        $this->assertCount(4, $all);
        $this->assertSame(array_slice($all, 1, 2), $query->searchPublished('window story', 'en', 1, 2)->pluck('slug')->all());
    }

    public function test_announcement_search_uses_the_explicit_locale_not_the_global_one(): void
    {
        App::setLocale('en');
        $announcement = SiteAnnouncement::factory()->create([
            'title' => 'Planned maintenance',
            'body' => 'Source notice body',
            'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
            'action_url' => '/status',
            'starts_at' => now()->subMinute(),
            'ends_at' => now()->addHour(),
        ]);
        $this->translate(EditorialContentType::SiteAnnouncement, $announcement->id, $announcement->updated_at, 'Planowana przerwa', 'Polska tresc ogloszenia');

        $query = app(ActiveAnnouncementQuery::class);

        $polish = $query->searchActive('przerwa', 'pl');
        $this->assertCount(1, $polish);
        $this->assertSame('Planowana przerwa', $polish->first()?->title);
        $this->assertCount(0, $query->searchActive('przerwa', 'en'));
        $this->assertCount(1, $query->searchActive('maintenance', 'en'));
    }

    public function test_event_search_is_not_limited_to_the_two_hundred_row_calendar_window(): void
    {
        $at = CarbonImmutable::parse('2026-07-24 12:00:00', 'UTC');
        foreach (range(1, 205) as $number) {
            $this->createEvent('filler-'.$number, 'Filler event '.$number, $at->addDays($number));
        }
        $this->createEvent('late-needle', 'Zeppelin regatta', $at->addDays(400));

        $hits = app(EventCalendarQuery::class)->searchPublic('en', 'zeppelin', 1, 5, $at);

        $this->assertCount(1, $hits);
        $this->assertSame('late-needle', $hits[0]['slug']);
    }

    private function translate(EditorialContentType $type, int $id, mixed $sourceUpdatedAt, string $title, string $body): void
    {
        EditorialTranslation::query()->create([
            'content_type' => $type->value,
            'content_id' => $id,
            'locale' => 'pl',
            'title' => $title,
            'body' => $body,
            'source_updated_at' => $sourceUpdatedAt,
            'published_at' => now()->subSecond(),
        ]);
    }

    private function createEvent(string $slug, string $title, CarbonImmutable $startsAt): void
    {
        $event = Event::factory()->create([
            'status' => Event::STATUS_SCHEDULED,
            'starts_at' => $startsAt,
            'ends_at' => $startsAt->addHour(),
            'featured' => false,
        ]);
        EventTranslation::factory()->create([
            'event_id' => $event->id,
            'locale' => 'en',
            'slug' => $slug,
            'title' => $title,
            'summary' => $title.' summary',
        ]);
    }
}
