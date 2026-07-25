<?php

namespace App\Admin;

final class AdminPermission
{
    public const ACCESS = 'admin.access';

    public const MANAGE_ROLES = 'admin.roles.manage';

    public const VIEW_AUDIT = 'audit.view';

    public const MANAGE_NEWS = 'cms.news.manage';

    public const MANAGE_PAGES = 'cms.pages.manage';

    public const MANAGE_MEDIA = 'media.manage';

    public const PORTAL_ACCESS = 'portal.access';

    public const MANAGE_PORTAL_ANNOUNCEMENTS = 'portal.announcements.manage';

    public const MANAGE_PORTAL_SETTINGS = 'portal.settings.manage';

    public const MANAGE_DOWNLOADS = 'downloads.manage';

    public const MANAGE_EVENTS = 'events.manage';

    public const PUBLISH_EVENTS = 'events.publish';

    public const MANAGE_SUPPORT_CONTENT = 'support.content.manage';

    public const WIKI_ACCESS = 'wiki.access';

    public const MANAGE_WIKI_ARTICLES = 'wiki.articles.manage';

    public const MANAGE_WIKI_CATEGORIES = 'wiki.categories.manage';

    public const PUBLISH_WIKI = 'wiki.publish';

    /**
     * @return list<string>
     */
    public static function all(): array
    {
        return [
            self::ACCESS,
            self::MANAGE_ROLES,
            self::VIEW_AUDIT,
            self::MANAGE_NEWS,
            self::MANAGE_PAGES,
            self::MANAGE_MEDIA,
            self::PORTAL_ACCESS,
            self::MANAGE_PORTAL_ANNOUNCEMENTS,
            self::MANAGE_PORTAL_SETTINGS,
            self::MANAGE_DOWNLOADS,
            self::MANAGE_EVENTS,
            self::PUBLISH_EVENTS,
            self::MANAGE_SUPPORT_CONTENT,
            self::WIKI_ACCESS,
            self::MANAGE_WIKI_ARTICLES,
            self::MANAGE_WIKI_CATEGORIES,
            self::PUBLISH_WIKI,
        ];
    }
}
