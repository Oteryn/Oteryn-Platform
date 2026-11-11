<?php

namespace App\Cms\Editorial;

enum EditorialContentType: string
{
    case NewsPost = 'news_post';
    case ManagedPage = 'managed_page';
    case SiteAnnouncement = 'site_announcement';
    case ClientRelease = 'client_release';

    public function requiresTitle(): bool
    {
        return $this !== self::ClientRelease;
    }
}
