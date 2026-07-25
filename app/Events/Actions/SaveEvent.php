<?php

namespace App\Events\Actions;

use App\Audit\AdminAuditRecorder;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\Identity\Models\Identity;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use DomainException;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

final class SaveEvent
{
    public function __construct(private readonly AdminAuditRecorder $audit) {}

    /**
     * @param  array<string, array{title: string, slug: string, summary: string, body: string}>  $translations
     */
    public function execute(
        Identity $actor,
        ?Event $event,
        DateTimeInterface $startsAt,
        DateTimeInterface $endsAt,
        bool $featured,
        ?int $newsPostId,
        array $translations,
        ?int $expectedLockVersion,
    ): Event {
        $normalizedStartsAt = CarbonImmutable::instance($startsAt)->utc();
        $normalizedEndsAt = CarbonImmutable::instance($endsAt)->utc();
        $normalizedTranslations = $this->normalizeTranslations($translations);

        if (! $normalizedEndsAt->isAfter($normalizedStartsAt)) {
            throw new InvalidArgumentException('The event end must be after its start.');
        }

        return DB::transaction(function () use (
            $actor,
            $event,
            $normalizedStartsAt,
            $normalizedEndsAt,
            $featured,
            $newsPostId,
            $normalizedTranslations,
            $expectedLockVersion,
        ): Event {
            $created = $event === null;

            if ($created) {
                $current = new Event;
                $current->created_by = $actor->id;
                $current->lock_version = 1;
            } else {
                $current = Event::query()->lockForUpdate()->findOrFail($event->id);

                if ($expectedLockVersion === null || $current->lock_version !== $expectedLockVersion) {
                    throw new DomainException('This event changed after the form was opened. Reload it before saving.');
                }

                $current->lock_version = $current->lock_version + 1;
            }

            $current->fill([
                'status' => Event::STATUS_DRAFT,
                'starts_at' => $normalizedStartsAt,
                'ends_at' => $normalizedEndsAt,
                'featured' => $featured,
                'news_post_id' => $newsPostId,
                'updated_by' => $actor->id,
                'published_by' => null,
            ]);
            $current->save();

            EventTranslation::query()
                ->where('event_id', $current->id)
                ->whereNotIn('locale', array_keys($normalizedTranslations))
                ->delete();

            foreach ($normalizedTranslations as $locale => $translation) {
                EventTranslation::query()->updateOrCreate(
                    [
                        'event_id' => $current->id,
                        'locale' => $locale,
                    ],
                    $translation,
                );
            }

            $this->audit->record(
                $actor->id,
                $created ? 'events.event_created' : 'events.event_updated',
                'event',
                (string) $current->id,
                [
                    'status' => $current->status,
                    'featured' => $current->featured,
                    'has_news' => $current->news_post_id !== null,
                    'locales' => implode(',', array_keys($normalizedTranslations)),
                    'starts_at' => $current->starts_at->toIso8601String(),
                    'ends_at' => $current->ends_at->toIso8601String(),
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);
    }

    /**
     * @param  array<string, array{title: string, slug: string, summary: string, body: string}>  $translations
     * @return array<string, array{title: string, slug: string, summary: string, body: string}>
     */
    private function normalizeTranslations(array $translations): array
    {
        if (! isset($translations['en'])) {
            throw new InvalidArgumentException('An English event translation is required.');
        }

        $normalized = [];

        foreach ($translations as $locale => $translation) {
            if (! in_array($locale, ['en', 'pl'], true)) {
                throw new InvalidArgumentException('Unsupported event translation locale.');
            }

            $title = trim($translation['title']);
            $slug = trim($translation['slug']);
            $summary = trim($translation['summary']);
            $body = trim($translation['body']);

            if ($title === '' || $slug === '' || $summary === '' || $body === '') {
                throw new InvalidArgumentException('Event translations must be complete.');
            }

            $normalized[$locale] = [
                'title' => $title,
                'slug' => $slug,
                'summary' => $summary,
                'body' => $body,
            ];
        }

        return $normalized;
    }
}
