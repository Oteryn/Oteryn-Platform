<?php

namespace App\PublicPortal;

use App\Announcements\Queries\AnnouncementTickerProvider;
use App\Cms\Models\NewsPost;
use App\Cms\PublicNewsQuery;
use App\Events\Queries\UpcomingEventProvider;
use App\PublicGameData\CanaryChannelRuntimeService;
use App\PublicGameData\CanaryGameDataRepository;
use App\PublicPortal\ViewModels\HomeNewsSummary;
use App\PublicPortal\ViewModels\HomePageViewModel;
use App\PublicPortal\ViewModels\HomeWorldChannel;
use App\PublicPortal\ViewModels\HomeWorldSummary;
use stdClass;
use Throwable;
use UnexpectedValueException;

final readonly class HomePageQuery
{
    public function __construct(
        private PublicNewsQuery $news,
        private CanaryGameDataRepository $gameData,
        private CanaryChannelRuntimeService $runtime,
        private AnnouncementTickerProvider $announcements,
        private UpcomingEventProvider $events,
    ) {}

    public function get(): HomePageViewModel
    {
        return new HomePageViewModel(
            world: $this->worldSummary(),
            news: $this->newsSummary(),
            announcements: $this->announcements->get(),
            upcomingEvent: $this->events->get(app()->getLocale()),
        );
    }

    private function newsSummary(): HomeNewsSummary
    {
        try {
            $posts = $this->news->latestPublished();
        } catch (Throwable) {
            return new HomeNewsSummary(PublicContentState::UNAVAILABLE, []);
        }

        if ($posts->isEmpty()) {
            return new HomeNewsSummary(PublicContentState::EMPTY, []);
        }

        /** @var list<NewsPost> $latestPosts */
        $latestPosts = array_values($posts->all());

        return new HomeNewsSummary(PublicContentState::AVAILABLE, $latestPosts);
    }

    private function worldSummary(): HomeWorldSummary
    {
        try {
            $channels = $this->gameData->configuredChannels();

            if ($channels->isEmpty()) {
                return new HomeWorldSummary(PublicContentState::EMPTY, [], null);
            }

            /** @var list<int> $channelIds */
            $channelIds = $channels
                ->map(fn (stdClass $channel): int => $this->channelId($channel->id ?? null))
                ->values()
                ->all();
            $runtimeSnapshot = $this->runtime->snapshot($channelIds);
            $state = $runtimeSnapshot->available
                ? PublicContentState::AVAILABLE
                : PublicContentState::UNAVAILABLE;
            $playersOnline = $runtimeSnapshot->available ? 0 : null;
            $worldChannels = [];

            foreach ($channels as $channel) {
                $channelId = $this->channelId($channel->id ?? null);
                $runtime = $runtimeSnapshot->available
                    ? $runtimeSnapshot->forChannel($channelId)
                    : null;
                $maintenanceMessage = $channel->maintenance_message ?? null;

                if ($runtimeSnapshot->available && $runtime === null) {
                    $state = PublicContentState::STALE;
                    $playersOnline = null;
                } elseif ($playersOnline !== null && $runtime !== null) {
                    $playersOnline += $runtime->playersOnline;
                }

                $worldChannels[] = new HomeWorldChannel(
                    id: $channelId,
                    name: $this->requiredString($channel->name ?? null, 'name'),
                    pvpType: $this->requiredString($channel->pvp_type ?? null, 'pvp_type'),
                    maxPlayers: $this->nonNegativeInteger($channel->max_players ?? null, 'max_players'),
                    maintenance: $this->booleanValue($channel->maintenance ?? null, 'maintenance'),
                    maintenanceMessage: is_string($maintenanceMessage) ? $maintenanceMessage : null,
                    runtimeStatus: $runtime?->status,
                    playersOnline: $runtime?->playersOnline,
                );
            }

            return new HomeWorldSummary($state, $worldChannels, $playersOnline);
        } catch (Throwable) {
            return new HomeWorldSummary(PublicContentState::UNAVAILABLE, [], null);
        }
    }

    private function channelId(mixed $channelId): int
    {
        if (is_int($channelId) && $channelId > 0) {
            return $channelId;
        }

        if (is_string($channelId) && ctype_digit($channelId) && (int) $channelId > 0) {
            return (int) $channelId;
        }

        throw new UnexpectedValueException('Configured Canary channel ID is invalid.');
    }

    private function requiredString(mixed $value, string $field): string
    {
        if (is_string($value) && trim($value) !== '') {
            return $value;
        }

        throw new UnexpectedValueException("Configured Canary channel {$field} is invalid.");
    }

    private function nonNegativeInteger(mixed $value, string $field): int
    {
        if (is_int($value) && $value >= 0) {
            return $value;
        }

        if (is_string($value) && ctype_digit($value)) {
            $parsed = filter_var($value, FILTER_VALIDATE_INT, [
                'options' => ['min_range' => 0],
            ]);

            if (is_int($parsed)) {
                return $parsed;
            }
        }

        throw new UnexpectedValueException("Configured Canary channel {$field} is invalid.");
    }

    private function booleanValue(mixed $value, string $field): bool
    {
        if (is_bool($value)) {
            return $value;
        }

        if ($value === 0 || $value === '0') {
            return false;
        }

        if ($value === 1 || $value === '1') {
            return true;
        }

        throw new UnexpectedValueException("Configured Canary channel {$field} is invalid.");
    }
}
