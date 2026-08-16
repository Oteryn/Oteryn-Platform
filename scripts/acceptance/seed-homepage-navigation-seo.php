<?php

declare(strict_types=1);

use App\Announcements\Models\SiteAnnouncement;
use App\Cms\Editorial\EditorialContentType;
use App\Events\Models\Event;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$now = now();

DB::transaction(function () use ($now): void {
    DB::table('site_announcements')->where('title', 'Acceptance realm maintenance')->delete();
    DB::table('site_announcements')->insert([
        'title' => 'Acceptance realm maintenance',
        'body' => 'A deterministic published announcement for homepage acceptance.',
        'severity' => SiteAnnouncement::SEVERITY_MAINTENANCE,
        'starts_at' => $now->copy()->subHour(),
        'ends_at' => $now->copy()->addDay(),
        'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
        'action_label' => null,
        'action_url' => null,
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);

    $eventIds = DB::table('event_translations')
        ->where(function ($query): void {
            $query
                ->where(function ($translation): void {
                    $translation
                        ->where('locale', 'en')
                        ->where('slug', 'acceptance-tournament');
                })
                ->orWhere('slug', 'like', 'content-scale-event-%');
        })
        ->pluck('event_id')
        ->map(static fn (mixed $id): int => (int) $id)
        ->unique()
        ->values()
        ->all();

    if ($eventIds !== []) {
        DB::table('event_translations')->whereIn('event_id', $eventIds)->delete();
        DB::table('events')->whereIn('id', $eventIds)->delete();
    }

    $eventId = DB::table('events')->insertGetId([
        'status' => Event::STATUS_SCHEDULED,
        'starts_at' => $now->copy()->addDay(),
        'ends_at' => $now->copy()->addDay()->addHours(2),
        'featured' => true,
        'news_post_id' => null,
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('event_translations')->insert([
        [
            'event_id' => $eventId,
            'locale' => 'en',
            'title' => 'Acceptance tournament',
            'slug' => 'acceptance-tournament',
            'summary' => 'A deterministic upcoming event for homepage acceptance.',
            'body' => 'Acceptance event details.',
            'created_at' => $now,
            'updated_at' => $now,
        ],
        [
            'event_id' => $eventId,
            'locale' => 'pl',
            'title' => 'Turniej testowy',
            'slug' => 'turniej-testowy',
            'summary' => 'Deterministyczne nadchodzące wydarzenie testowe.',
            'body' => 'Szczegóły wydarzenia testowego.',
            'created_at' => $now,
            'updated_at' => $now,
        ],
    ]);

    $todayAnnouncementId = DB::table('site_announcements')->where('title', 'Acceptance Today maintenance')->value('id');
    if ($todayAnnouncementId !== null) {
        DB::table('editorial_translations')
            ->where('content_type', EditorialContentType::SiteAnnouncement->value)
            ->where('content_id', $todayAnnouncementId)
            ->delete();
        DB::table('site_announcements')->where('id', $todayAnnouncementId)->delete();
    }
    $todayAnnouncementId = DB::table('site_announcements')->insertGetId([
        'title' => 'Acceptance Today maintenance',
        'body' => 'A deterministic public Today announcement.',
        'severity' => SiteAnnouncement::SEVERITY_MAINTENANCE,
        'starts_at' => $now->copy()->subMinutes(30),
        'ends_at' => $now->copy()->addHours(12),
        'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
        'action_label' => null,
        'action_url' => null,
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('editorial_translations')->insert([
        'content_type' => EditorialContentType::SiteAnnouncement->value,
        'content_id' => $todayAnnouncementId,
        'locale' => 'pl',
        'title' => 'Testowa konserwacja Dzisiaj',
        'body' => 'Deterministyczny publiczny komunikat dla widoku Dzisiaj.',
        'action_label' => null,
        'source_updated_at' => $now,
        'published_at' => $now->copy()->subMinute(),
        'created_at' => $now,
        'updated_at' => $now,
    ]);

    $todayEventIds = DB::table('event_translations')
        ->whereIn('slug', ['acceptance-today-event', 'testowe-wydarzenie-dzisiaj'])
        ->pluck('event_id')
        ->map(static fn (mixed $id): int => (int) $id)
        ->unique()
        ->values()
        ->all();
    if ($todayEventIds !== []) {
        DB::table('event_translations')->whereIn('event_id', $todayEventIds)->delete();
        DB::table('events')->whereIn('id', $todayEventIds)->delete();
    }
    $todayEventId = DB::table('events')->insertGetId([
        'status' => Event::STATUS_SCHEDULED,
        'starts_at' => $now->copy()->addHours(6),
        'ends_at' => $now->copy()->addHours(8),
        'featured' => true,
        'news_post_id' => null,
        'lock_version' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('event_translations')->insert([
        [
            'event_id' => $todayEventId,
            'locale' => 'en',
            'title' => 'Acceptance Today event',
            'slug' => 'acceptance-today-event',
            'summary' => 'A deterministic public Today event.',
            'body' => 'Acceptance Today event details.',
            'created_at' => $now,
            'updated_at' => $now,
        ],
        [
            'event_id' => $todayEventId,
            'locale' => 'pl',
            'title' => 'Testowe wydarzenie Dzisiaj',
            'slug' => 'testowe-wydarzenie-dzisiaj',
            'summary' => 'Deterministyczne publiczne wydarzenie dla widoku Dzisiaj.',
            'body' => 'Szczegóły testowego wydarzenia Dzisiaj.',
            'created_at' => $now,
            'updated_at' => $now,
        ],
    ]);

    $todayNewsId = DB::table('news_posts')->where('slug', 'acceptance-today-news')->value('id');
    if ($todayNewsId !== null) {
        DB::table('editorial_translations')
            ->where('content_type', EditorialContentType::NewsPost->value)
            ->where('content_id', $todayNewsId)
            ->delete();
        DB::table('news_posts')->where('id', $todayNewsId)->delete();
    }
    $todayNewsId = DB::table('news_posts')->insertGetId([
        'slug' => 'acceptance-today-news',
        'title' => 'Acceptance Today update',
        'body' => 'A deterministic published update for the public Today page.',
        'published_at' => $now->copy()->subMinutes(2),
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('editorial_translations')->insert([
        'content_type' => EditorialContentType::NewsPost->value,
        'content_id' => $todayNewsId,
        'locale' => 'pl',
        'title' => 'Testowa aktualność Dzisiaj',
        'body' => 'Deterministyczna opublikowana aktualność dla publicznego widoku Dzisiaj.',
        'action_label' => null,
        'source_updated_at' => $now,
        'published_at' => $now->copy()->subMinute(),
        'created_at' => $now,
        'updated_at' => $now,
    ]);
});

fwrite(STDOUT, "acceptance-state: homepage navigation SEO and public Today seeded\n");
