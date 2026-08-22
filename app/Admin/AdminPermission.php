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

    public const MANAGE_SUPPORT_TICKETS = 'support.tickets.manage';

    public const MANAGE_SUPPORT_REPORTS = 'support.reports.manage';

    public const MANAGE_SUPPORT_ENFORCEMENT = 'support.enforcement.manage';

    public const MANAGE_MARKETPLACE = 'marketplace.manage';

    public const RECONCILE_PAYMENTS = 'payments.reconcile';

    public const GAME_CATALOG_ACCESS = 'game_catalog.access';

    public const VIEW_GAME_CATALOG_SNAPSHOTS = 'game_catalog.snapshots.view';

    public const IMPORT_GAME_CATALOG_SNAPSHOTS = 'game_catalog.snapshots.import';

    public const ACTIVATE_GAME_CATALOG_SNAPSHOTS = 'game_catalog.snapshots.activate';

    public const MANAGE_GAME_CATALOG_PROFILES = 'game_catalog.profiles.manage';

    public const MANAGE_GAME_CATALOG_TRANSLATIONS = 'game_catalog.translations.manage';

    public const MANAGE_GAME_CATALOG_OVERRIDES = 'game_catalog.overrides.manage';

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
            self::MANAGE_SUPPORT_TICKETS,
            self::MANAGE_SUPPORT_REPORTS,
            self::MANAGE_SUPPORT_ENFORCEMENT,
            self::MANAGE_MARKETPLACE,
            self::RECONCILE_PAYMENTS,
            self::GAME_CATALOG_ACCESS,
            self::VIEW_GAME_CATALOG_SNAPSHOTS,
            self::IMPORT_GAME_CATALOG_SNAPSHOTS,
            self::ACTIVATE_GAME_CATALOG_SNAPSHOTS,
            self::MANAGE_GAME_CATALOG_PROFILES,
            self::MANAGE_GAME_CATALOG_TRANSLATIONS,
            self::MANAGE_GAME_CATALOG_OVERRIDES,
            self::WIKI_ACCESS,
            self::MANAGE_WIKI_ARTICLES,
            self::MANAGE_WIKI_CATEGORIES,
            self::PUBLISH_WIKI,
        ];
    }
}
