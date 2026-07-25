<?php

namespace App\Announcements\Queries;

use App\Announcements\Models\SiteAnnouncement;
use App\Announcements\ViewModels\AnnouncementTicker;
use App\PublicPortal\PublicContentState;
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
            return new AnnouncementTicker(PublicContentState::UNAVAILABLE, []);
        }

        if ($items->isEmpty()) {
            return new AnnouncementTicker(PublicContentState::EMPTY, []);
        }

        /** @var list<SiteAnnouncement> $list */
        $list = array_values($items->all());

        return new AnnouncementTicker(PublicContentState::AVAILABLE, $list);
    }

    public function render(?DateTimeInterface $readTime = null, int $limit = 5): View
    {
        return view('announcements.components.ticker', [
            'ticker' => $this->get($readTime, $limit),
        ]);
    }
}
