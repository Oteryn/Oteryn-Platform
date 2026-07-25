<?php

namespace App\Events\Queries;

use App\Events\ViewModels\UpcomingEventSummary;
use App\PublicPortal\PublicContentState;
use DateTimeInterface;
use Illuminate\Contracts\View\View;
use Throwable;

final class UpcomingEventProvider
{
    public function __construct(private readonly EventCalendarQuery $events) {}

    public function get(?string $locale = null, ?DateTimeInterface $readTime = null): UpcomingEventSummary
    {
        try {
            $event = $this->events->upcomingSummary($locale ?? app()->getLocale(), $readTime);
        } catch (Throwable) {
            return new UpcomingEventSummary(PublicContentState::UNAVAILABLE, null);
        }

        if ($event === null) {
            return new UpcomingEventSummary(PublicContentState::EMPTY, null);
        }

        return new UpcomingEventSummary(PublicContentState::AVAILABLE, $event);
    }

    public function render(?string $locale = null, ?DateTimeInterface $readTime = null): View
    {
        return view('events.components.upcoming-summary', [
            'summary' => $this->get($locale, $readTime),
        ]);
    }
}
