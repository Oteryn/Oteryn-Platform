<?php

namespace App\PublicPortal\ViewModels;

use App\Announcements\ViewModels\AnnouncementTicker;
use App\Events\ViewModels\UpcomingEventSummary;

final readonly class HomePageViewModel
{
    public function __construct(
        public HomeWorldSummary $world,
        public HomeNewsSummary $news,
        public AnnouncementTicker $announcements,
        public UpcomingEventSummary $upcomingEvent,
    ) {}
}
