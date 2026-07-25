<?php

namespace App\Announcements\ViewModels;

use App\Announcements\Models\SiteAnnouncement;
use App\PublicPortal\PublicContentState;

final readonly class AnnouncementTicker
{
    /** @var list<SiteAnnouncement> */
    public array $items;

    /**
     * @param  list<SiteAnnouncement>  $items
     */
    public function __construct(
        public PublicContentState $state,
        array $items,
    ) {
        $this->items = $items;
    }
}
