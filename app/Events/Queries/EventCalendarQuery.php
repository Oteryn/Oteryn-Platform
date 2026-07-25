<?php

namespace App\Events\Queries;

use App\Events\Models\Event;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use Illuminate\Database\Query\JoinClause;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use UnexpectedValueException;

/**
 * @phpstan-type EventSummary array{id: int, title: string, slug: string, summary: string, starts_at: CarbonImmutable, ends_at: CarbonImmutable, status: string, featured: bool}
 * @phpstan-type EventCalendar array{active: list<EventSummary>, upcoming: list<EventSummary>, archived: list<EventSummary>, cancelled: list<EventSummary>}
 * @phpstan-type EventRow array{id: mixed, status: mixed, starts_at: mixed, ends_at: mixed, featured: mixed, title: mixed, slug: mixed, summary: mixed}
 * @phpstan-type EventDetailRow array{id: mixed, status: mixed, starts_at: mixed, ends_at: mixed, featured: mixed, title: mixed, slug: mixed, summary: mixed, body: mixed, news_slug: mixed, news_title: mixed}
 */
final class EventCalendarQuery
{
    /**
     * @return EventCalendar
     */
    public function calendar(string $locale, ?DateTimeInterface $readTime = null): array
    {
        $this->assertLocale($locale);
        $at = CarbonImmutable::instance($readTime ?? now())->utc();

        $rows = DB::table('events')
            ->join('event_translations', function (JoinClause $join) use ($locale): void {
                $join
                    ->on('event_translations.event_id', '=', 'events.id')
                    ->where('event_translations.locale', '=', $locale);
            })
            ->whereIn('events.status', Event::publicStatuses())
            ->orderByDesc('events.featured')
            ->orderBy('events.starts_at')
            ->orderBy('events.id')
            ->limit(200)
            ->get([
                'events.id',
                'events.status',
                'events.starts_at',
                'events.ends_at',
                'events.featured',
                'event_translations.title',
                'event_translations.slug',
                'event_translations.summary',
            ]);

        /** @var EventCalendar $calendar */
        $calendar = [
            'active' => [],
            'upcoming' => [],
            'archived' => [],
            'cancelled' => [],
        ];

        foreach ($rows as $row) {
            /** @var EventRow $data */
            $data = (array) $row;
            $summary = $this->summary($data, $at);

            switch ($summary['status']) {
                case Event::STATUS_ACTIVE:
                    $calendar['active'][] = $summary;
                    break;
                case Event::STATUS_SCHEDULED:
                    $calendar['upcoming'][] = $summary;
                    break;
                case Event::STATUS_COMPLETED:
                    $calendar['archived'][] = $summary;
                    break;
                case Event::STATUS_CANCELLED:
                    $calendar['cancelled'][] = $summary;
                    break;
                default:
                    throw new UnexpectedValueException('Unexpected public event state.');
            }
        }

        usort($calendar['archived'], self::compareArchived(...));
        usort($calendar['cancelled'], self::compareCancelled(...));

        return $calendar;
    }

    /**
     * @return array{
     *     id: int,
     *     title: string,
     *     slug: string,
     *     summary: string,
     *     body: string,
     *     starts_at: CarbonImmutable,
     *     ends_at: CarbonImmutable,
     *     status: string,
     *     featured: bool,
     *     news_slug: string|null,
     *     news_title: string|null
     * }|null
     */
    public function findPublishedBySlug(
        string $locale,
        string $slug,
        ?DateTimeInterface $readTime = null,
    ): ?array {
        $this->assertLocale($locale);
        $at = CarbonImmutable::instance($readTime ?? now())->utc();

        $row = DB::table('events')
            ->join('event_translations', function (JoinClause $join) use ($locale): void {
                $join
                    ->on('event_translations.event_id', '=', 'events.id')
                    ->where('event_translations.locale', '=', $locale);
            })
            ->leftJoin('news_posts', function (JoinClause $join) use ($at): void {
                $join
                    ->on('news_posts.id', '=', 'events.news_post_id')
                    ->whereNotNull('news_posts.published_at')
                    ->where('news_posts.published_at', '<=', $at);
            })
            ->whereIn('events.status', Event::publicStatuses())
            ->where('event_translations.slug', $slug)
            ->first([
                'events.id',
                'events.status',
                'events.starts_at',
                'events.ends_at',
                'events.featured',
                'event_translations.title',
                'event_translations.slug',
                'event_translations.summary',
                'event_translations.body',
                'news_posts.slug as news_slug',
                'news_posts.title as news_title',
            ]);

        if ($row === null) {
            return null;
        }

        /** @var EventDetailRow $data */
        $data = (array) $row;
        $summary = $this->summary($data, $at);

        return [
            ...$summary,
            'body' => $this->stringValue($data['body'], 'body'),
            'news_slug' => $this->nullableStringValue($data['news_slug'], 'news_slug'),
            'news_title' => $this->nullableStringValue($data['news_title'], 'news_title'),
        ];
    }

    /**
     * @return EventSummary|null
     */
    public function upcomingSummary(string $locale, ?DateTimeInterface $readTime = null): ?array
    {
        $calendar = $this->calendar($locale, $readTime);

        return $calendar['active'][0] ?? $calendar['upcoming'][0] ?? null;
    }

    /**
     * @param  EventRow|EventDetailRow  $row
     * @return EventSummary
     */
    private function summary(array $row, CarbonImmutable $at): array
    {
        $startsAt = CarbonImmutable::parse($this->stringValue($row['starts_at'], 'starts_at'), 'UTC');
        $endsAt = CarbonImmutable::parse($this->stringValue($row['ends_at'], 'ends_at'), 'UTC');
        $persistedStatus = $this->stringValue($row['status'], 'status');

        if (! in_array($persistedStatus, Event::publicStatuses(), true)) {
            throw new UnexpectedValueException('Unexpected persisted event state.');
        }

        $status = match (true) {
            $persistedStatus === Event::STATUS_CANCELLED => Event::STATUS_CANCELLED,
            $at->lt($startsAt) => Event::STATUS_SCHEDULED,
            $at->greaterThanOrEqualTo($endsAt) => Event::STATUS_COMPLETED,
            default => Event::STATUS_ACTIVE,
        };

        return [
            'id' => $this->integerValue($row['id'], 'id'),
            'title' => $this->stringValue($row['title'], 'title'),
            'slug' => $this->stringValue($row['slug'], 'slug'),
            'summary' => $this->stringValue($row['summary'], 'summary'),
            'starts_at' => $startsAt,
            'ends_at' => $endsAt,
            'status' => $status,
            'featured' => $this->booleanValue($row['featured'], 'featured'),
        ];
    }

    /**
     * @param  EventSummary  $left
     * @param  EventSummary  $right
     */
    private static function compareArchived(array $left, array $right): int
    {
        return $right['ends_at']->getTimestamp() <=> $left['ends_at']->getTimestamp();
    }

    /**
     * @param  EventSummary  $left
     * @param  EventSummary  $right
     */
    private static function compareCancelled(array $left, array $right): int
    {
        return $right['starts_at']->getTimestamp() <=> $left['starts_at']->getTimestamp();
    }

    private function stringValue(mixed $value, string $field): string
    {
        if (! is_string($value)) {
            throw new UnexpectedValueException("Unexpected {$field} value.");
        }

        return $value;
    }

    private function nullableStringValue(mixed $value, string $field): ?string
    {
        if ($value === null) {
            return null;
        }

        return $this->stringValue($value, $field);
    }

    private function integerValue(mixed $value, string $field): int
    {
        if (is_int($value)) {
            return $value;
        }

        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new UnexpectedValueException("Unexpected {$field} value.");
    }

    private function booleanValue(mixed $value, string $field): bool
    {
        return match ($value) {
            true, 1, '1' => true,
            false, 0, '0' => false,
            default => throw new UnexpectedValueException("Unexpected {$field} value."),
        };
    }

    private function assertLocale(string $locale): void
    {
        if (! in_array($locale, ['en', 'pl'], true)) {
            throw new InvalidArgumentException('Unsupported event locale.');
        }
    }
}
