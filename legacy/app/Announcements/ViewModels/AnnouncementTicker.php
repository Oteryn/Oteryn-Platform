<?php

namespace App\Announcements\ViewModels;

use App\Announcements\Models\SiteAnnouncement;

final readonly class AnnouncementTicker
{
    /** @var list<SiteAnnouncement> */
    public array $items;

    /**
     * @param  list<SiteAnnouncement>  $items
     */
    public function __construct(
        public AnnouncementTickerState $state,
        array $items,
    ) {
        $this->items = $items;
    }
}
