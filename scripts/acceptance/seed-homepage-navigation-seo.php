<?php

declare(strict_types=1);

use App\Announcements\Models\SiteAnnouncement;
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
});

fwrite(STDOUT, "acceptance-state: homepage navigation and SEO seeded\n");
