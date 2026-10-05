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
$liveOpsWorldId = '018f0f1e-7b2c-7a31-8d4e-1234567890ab';
$liveOpsChannelId = '018f0f1e-7b2c-7a32-8d4e-1234567890ac';
$liveOpsNodeIdentity = 'CN=acceptance-runtime-node';

DB::transaction(function () use ($now, $liveOpsWorldId, $liveOpsChannelId, $liveOpsNodeIdentity): void {
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

    DB::table('native_runtime_status_reports')->where('world_id', $liveOpsWorldId)->delete();
    DB::table('native_scope_assignments')->where('world_id', $liveOpsWorldId)->delete();
    DB::table('game_channels')->where('channel_id', $liveOpsChannelId)->delete();
    DB::table('game_worlds')->where('world_id', $liveOpsWorldId)->delete();

    $liveOpsWorldRowId = DB::table('game_worlds')->insertGetId([
        'world_id' => $liveOpsWorldId,
        'slug' => 'acceptance-liveops',
        'name' => 'Acceptance LiveOps',
        'region' => 'TEST',
        'status' => 'online',
        'login_enabled' => true,
        'game_host' => '127.0.0.1',
        'game_port' => 7172,
        'gameplay_policy_revision' => 1,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('game_channels')->insert([
        'game_world_id' => $liveOpsWorldRowId,
        'channel_id' => $liveOpsChannelId,
        'channel_key' => 'acceptance-primary',
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('native_scope_assignments')->insert([
        'world_id' => $liveOpsWorldId,
        'channel_id' => $liveOpsChannelId,
        'assignment_epoch' => '1',
        'ownership_generation' => '1',
        'node_identity' => $liveOpsNodeIdentity,
        'assigned_at' => $now->copy()->subSeconds(3)->getTimestamp(),
        'created_at' => $now,
        'updated_at' => $now,
    ]);
    DB::table('native_runtime_status_reports')->insert([
        'world_id' => $liveOpsWorldId,
        'channel_id' => $liveOpsChannelId,
        'node_identity' => $liveOpsNodeIdentity,
        'source_authority' => 'oteryn-game',
        'node_id' => '11111111-2222-3333-8444-555555555555',
        'assignment_epoch' => '1',
        'scope_ownership_generation' => '1',
        'source_revision' => '1',
        'decision_identity' => 'acceptance-runtime',
        'ready' => true,
        'published_at' => $now->copy()->subSeconds(3)->getTimestamp(),
        'observed_at' => $now->copy()->subSeconds(2)->getTimestamp(),
        'protocol_major' => 1,
        'transport_profile' => 1,
        'route_revision' => 'acceptance-route-1',
        'runtime_observation_revision' => 'acceptance-runtime-1',
        'ruleset_revision' => 'acceptance-ruleset-1',
        'content_revision' => 'acceptance-content-1',
        'map_revision' => 'acceptance-map-1',
        'world_policy_revision' => 'acceptance-policy-1',
        'offer_revision' => 'acceptance-offer-1',
        'content_digest' => str_repeat('b', 64),
        'invalid' => false,
        'created_at' => $now,
        'updated_at' => $now,
    ]);
});

fwrite(STDOUT, "acceptance-state: homepage navigation SEO, public Today and native LiveOps seeded\n");
