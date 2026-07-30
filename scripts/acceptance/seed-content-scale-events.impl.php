<?php

declare(strict_types=1);

use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$englishTitle = implode(' ', array_map(
    static fn (int $index): string => sprintf('EnglishEventSegment%02d', $index),
    range(1, 9),
));
$polishTitle = implode(' ', array_map(
    static fn (int $index): string => sprintf('PolskiSegmentWydarzenia%02d', $index),
    range(1, 8),
));
$englishSummary = 'English event summary boundary '.implode(' ', array_map(
    static fn (int $index): string => sprintf('summary-segment-%02d', $index),
    range(1, 24),
));
$polishSummary = 'Polskie podsumowanie wydarzenia '.implode(' ', array_map(
    static fn (int $index): string => sprintf('segment-podsumowania-%02d', $index),
    range(1, 24),
));
$englishBody = 'English event body boundary '.implode(' ', array_map(
    static fn (int $index): string => sprintf('event-body-segment-%03d', $index),
    range(1, 90),
));
$polishBody = 'Polska treść wydarzenia '.implode(' ', array_map(
    static fn (int $index): string => sprintf('segment-treści-wydarzenia-%03d', $index),
    range(1, 90),
));

$existingIds = EventTranslation::query()
    ->where('slug', 'like', 'content-scale-event-%')
    ->pluck('event_id')
    ->map(static fn (mixed $id): int => (int) $id)
    ->all();

DB::transaction(static function () use ($existingIds): void {
    if ($existingIds === []) {
        return;
    }

    EventTranslation::query()->whereIn('event_id', $existingIds)->delete();
    Event::query()->whereIn('id', $existingIds)->delete();
});

$now = now()->startOfMinute();
$scaleIds = [];

for ($index = 1; $index <= 26; $index++) {
    $event = Event::query()->create([
        'status' => Event::STATUS_DRAFT,
        'starts_at' => $now->copy()->addDays($index),
        'ends_at' => $now->copy()->addDays($index)->addHour(),
        'featured' => false,
        'news_post_id' => null,
        'lock_version' => 1,
    ]);
    $scaleIds[] = $event->id;

    EventTranslation::query()->create([
        'event_id' => $event->id,
        'locale' => 'en',
        'title' => sprintf('Content Scale Event %03d', $index),
        'slug' => sprintf('content-scale-event-%03d', $index),
        'summary' => sprintf('Bounded event pagination fixture %03d.', $index),
        'body' => sprintf('Bounded event body fixture %03d.', $index),
    ]);
}

$event = Event::query()->create([
    'status' => Event::STATUS_ACTIVE,
    'starts_at' => $now->copy()->subHour(),
    'ends_at' => $now->copy()->addDay(),
    'featured' => true,
    'news_post_id' => null,
    'lock_version' => 1,
]);

EventTranslation::query()->create([
    'event_id' => $event->id,
    'locale' => 'en',
    'title' => $englishTitle,
    'slug' => 'content-scale-event-en',
    'summary' => $englishSummary,
    'body' => $englishBody,
]);
EventTranslation::query()->create([
    'event_id' => $event->id,
    'locale' => 'pl',
    'title' => $polishTitle,
    'slug' => 'content-scale-event-pl',
    'summary' => $polishSummary,
    'body' => $polishBody,
]);
$event->touch();

fwrite(STDOUT, json_encode([
    'event_id' => $event->id,
    'event_english_slug' => 'content-scale-event-en',
    'event_polish_slug' => 'content-scale-event-pl',
    'event_english_title' => $englishTitle,
    'event_polish_title' => $polishTitle,
    'event_english_summary' => $englishSummary,
    'event_polish_summary' => $polishSummary,
    'event_english_body' => $englishBody,
    'event_polish_body' => $polishBody,
    'event_page_two_ids' => array_slice($scaleIds, 0, 2),
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE).PHP_EOL);
