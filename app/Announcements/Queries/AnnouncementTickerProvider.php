<?php

namespace App\Announcements\Queries;

use App\Announcements\Models\SiteAnnouncement;
use App\Announcements\ViewModels\AnnouncementTicker;
use App\Announcements\ViewModels\AnnouncementTickerState;
use DateTimeInterface;
use Illuminate\Contracts\View\View;
use Throwable;

final class AnnouncementTickerProvider
{
    public function __construct(private readonly ActiveAnnouncementQuery $announcements) {}

    public function get(?DateTimeInterface $readTime = null, int $limit = 5): AnnouncementTicker
    {
        try {
            $items = $this->announcements->active($limit, $readTime);
        } catch (Throwable) {
            return new AnnouncementTicker(AnnouncementTickerState::UNAVAILABLE, []);
        }

        if ($items->isEmpty()) {
            return new AnnouncementTicker(AnnouncementTickerState::EMPTY, []);
        }

        /** @var list<SiteAnnouncement> $list */
        $list = array_values($items->all());

        return new AnnouncementTicker(AnnouncementTickerState::AVAILABLE, $list);
    }

    public function render(?DateTimeInterface $readTime = null, int $limit = 5): View
    {
        return view('announcements.components.ticker', [
            'ticker' => $this->get($readTime, $limit),
        ]);
    }
}
