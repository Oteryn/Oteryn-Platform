<?php

namespace App\PublicPortal\Today;

use App\Announcements\Models\SiteAnnouncement;
use App\Announcements\Queries\AnnouncementTickerProvider;
use App\Announcements\ViewModels\AnnouncementTickerState;
use App\Cms\Models\NewsPost;
use App\Cms\PublicNewsQuery;
use App\Events\Queries\UpcomingEventProvider;
use App\Events\ViewModels\UpcomingEventState;
use Carbon\CarbonImmutable;
use DateTimeInterface;
use Illuminate\Support\Str;
use Throwable;

final readonly class TodayPageQuery
{
    private const CARD_LIMIT = 3;

    public function __construct(
        private AnnouncementTickerProvider $announcements,
        private UpcomingEventProvider $events,
        private PublicNewsQuery $news,
    ) {}

    public function get(?DateTimeInterface $readTime = null, ?string $validationScenario = null): TodayPageViewModel
    {
        $evaluatedAt = $readTime === null
            ? CarbonImmutable::now()
            : CarbonImmutable::instance($readTime);

        $cards = [
            $this->liveOpsCard($evaluatedAt),
            $this->announcementCard($evaluatedAt, $validationScenario),
            $this->eventCard($evaluatedAt, $validationScenario),
            $this->newsCard($evaluatedAt, $validationScenario),
        ];

        usort($cards, static fn (TodayCard $left, TodayCard $right): int => $left->priority <=> $right->priority);

        $availableProviders = array_filter(
            $cards,
            static fn (TodayCard $card): bool => $card->state !== TodayCardState::UNAVAILABLE,
        );
        $unavailableProviders = array_filter(
            $cards,
            static fn (TodayCard $card): bool => $card->state === TodayCardState::UNAVAILABLE,
        );

        $state = match (true) {
            $availableProviders === [] => TodayPageState::UNAVAILABLE,
            $unavailableProviders !== [] => TodayPageState::PARTIAL,
            default => TodayPageState::COMPLETE,
        };

        return new TodayPageViewModel($state, $cards);
    }

    private function liveOpsCard(DateTimeInterface $evaluatedAt): TodayCard
    {
        return new TodayCard(
            kind: 'liveops',
            sourceOwner: 'LiveOps',
            sourceIdentity: 'LiveOps.public-runtime-summary',
            canonicalSourceUrl: null,
            state: TodayCardState::UNAVAILABLE,
            priority: 10,
            evaluatedAt: $evaluatedAt,
        );
    }

    private function announcementCard(DateTimeInterface $evaluatedAt, ?string $scenario): TodayCard
    {
        if ($scenario === 'empty') {
            return $this->card('announcements', 'Announcements', 'Announcements.active-public', null, TodayCardState::EMPTY, 20, $evaluatedAt);
        }

        $ticker = $this->announcements->get($evaluatedAt, self::CARD_LIMIT);
        $state = match ($ticker->state) {
            AnnouncementTickerState::AVAILABLE => TodayCardState::PRESENT,
            AnnouncementTickerState::EMPTY => TodayCardState::EMPTY,
            AnnouncementTickerState::UNAVAILABLE => TodayCardState::UNAVAILABLE,
        };

        $items = [];
        if ($state === TodayCardState::PRESENT) {
            $items = array_map(
                static fn (SiteAnnouncement $announcement): TodayItem => new TodayItem(
                    publicId: 'announcement-'.$announcement->id,
                    title: $announcement->title,
                    summary: self::snippet($announcement->body),
                    url: $announcement->action_url,
                    actionLabel: $announcement->action_label,
                    effectiveAt: $announcement->starts_at,
                    badge: $announcement->severity,
                ),
                $ticker->items,
            );
        }

        return $this->card('announcements', 'Announcements', 'Announcements.active-public', null, $state, 20, $evaluatedAt, $items);
    }

    private function eventCard(DateTimeInterface $evaluatedAt, ?string $scenario): TodayCard
    {
        if ($scenario === 'empty') {
            return $this->card('events', 'Events', 'Events.upcoming-public', route('events.index'), TodayCardState::EMPTY, 30, $evaluatedAt);
        }

        $summary = $this->events->get(app()->getLocale(), $evaluatedAt);
        $state = match ($summary->state) {
            UpcomingEventState::AVAILABLE => TodayCardState::PRESENT,
            UpcomingEventState::EMPTY => TodayCardState::EMPTY,
            UpcomingEventState::UNAVAILABLE => TodayCardState::UNAVAILABLE,
        };

        $items = [];
        if ($state === TodayCardState::PRESENT && $summary->event !== null) {
            $event = $summary->event;
            $items[] = new TodayItem(
                publicId: 'event-'.$event['id'],
                title: $event['title'],
                summary: self::snippet($event['summary']),
                url: route('events.show', ['slug' => $event['slug']]),
                actionLabel: (string) __('today.actions.view_event'),
                effectiveAt: $event['starts_at'],
                badge: $event['featured'] ? (string) __('today.badges.featured') : null,
            );
        }

        return $this->card('events', 'Events', 'Events.upcoming-public', route('events.index'), $state, 30, $evaluatedAt, $items);
    }

    private function newsCard(DateTimeInterface $evaluatedAt, ?string $scenario): TodayCard
    {
        if ($scenario === 'empty') {
            return $this->card('news', 'CMS', 'CMS.news-latest-public', route('news.index'), TodayCardState::EMPTY, 40, $evaluatedAt);
        }
        if ($scenario === 'news-outage') {
            return $this->card('news', 'CMS', 'CMS.news-latest-public', route('news.index'), TodayCardState::UNAVAILABLE, 40, $evaluatedAt);
        }

        try {
            $posts = $this->news->latestPublished(self::CARD_LIMIT, $evaluatedAt);
        } catch (Throwable) {
            return $this->card('news', 'CMS', 'CMS.news-latest-public', route('news.index'), TodayCardState::UNAVAILABLE, 40, $evaluatedAt);
        }

        if ($posts->isEmpty()) {
            return $this->card('news', 'CMS', 'CMS.news-latest-public', route('news.index'), TodayCardState::EMPTY, 40, $evaluatedAt);
        }

        /** @var list<TodayItem> $items */
        $items = array_values($posts
            ->map(static fn (NewsPost $post): TodayItem => new TodayItem(
                publicId: 'news-'.$post->id,
                title: $post->title,
                summary: self::snippet($post->body),
                url: route('news.show', ['slug' => $post->slug]),
                actionLabel: (string) __('today.actions.read_news'),
                effectiveAt: $post->published_at,
            ))
            ->all());

        return $this->card('news', 'CMS', 'CMS.news-latest-public', route('news.index'), TodayCardState::PRESENT, 40, $evaluatedAt, $items);
    }

    /** @param list<TodayItem> $items */
    private function card(
        string $kind,
        string $owner,
        string $identity,
        ?string $sourceUrl,
        TodayCardState $state,
        int $priority,
        DateTimeInterface $evaluatedAt,
        array $items = [],
    ): TodayCard {
        return new TodayCard(
            kind: $kind,
            sourceOwner: $owner,
            sourceIdentity: $identity,
            canonicalSourceUrl: $sourceUrl,
            state: $state,
            priority: $priority,
            evaluatedAt: $evaluatedAt,
            items: $items,
        );
    }

    private static function snippet(?string $value): ?string
    {
        if ($value === null) {
            return null;
        }

        $normalized = Str::squish(strip_tags($value));

        return $normalized === '' ? null : Str::limit($normalized, 240, '…');
    }
}
