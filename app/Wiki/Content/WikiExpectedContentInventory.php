<?php

namespace App\Wiki\Content;

final class WikiExpectedContentInventory
{
    public const VERSION = '2026-08-10.1';

    public const CATALOG_VERSION = '2026-07-26.1';

    public const EFFECTIVE_FROM = '2026-08-10';

    /** @var list<string> */
    public const LOCALES = ['en', 'pl'];

    /**
     * The authoritative expected category set for the reviewed launch-content scope.
     *
     * @var array<string, array{sort_order: int, slugs: array{en: string, pl: string}}>
     */
    public const CATEGORIES = [
        'getting-started' => [
            'sort_order' => 10,
            'slugs' => ['en' => 'getting-started', 'pl' => 'pierwsze-kroki'],
        ],
        'server-information' => [
            'sort_order' => 20,
            'slugs' => ['en' => 'server-information', 'pl' => 'informacje-o-serwerze'],
        ],
        'game-systems' => [
            'sort_order' => 30,
            'slugs' => ['en' => 'game-systems', 'pl' => 'systemy-gry'],
        ],
        'support' => [
            'sort_order' => 40,
            'slugs' => ['en' => 'support', 'pl' => 'wsparcie'],
        ],
    ];

    /**
     * The English launch slug is the stable inventory key for this inventory version.
     * A slug change is therefore an explicit inventory-version change rather than an
     * accidental replacement that can still satisfy aggregate article counts.
     *
     * @var array<string, array{
     *     content_type: string,
     *     featured: bool,
     *     sort_order: int,
     *     category_keys: list<string>,
     *     slugs: array{en: string, pl: string}
     * }>
     */
    public const ARTICLES = [
        'download-and-installation' => [
            'content_type' => 'guide',
            'featured' => true,
            'sort_order' => 10,
            'category_keys' => ['getting-started'],
            'slugs' => ['en' => 'download-and-installation', 'pl' => 'pobieranie-i-instalacja'],
        ],
        'creating-an-account' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 20,
            'category_keys' => ['getting-started'],
            'slugs' => ['en' => 'creating-an-account', 'pl' => 'tworzenie-konta'],
        ],
        'creating-a-character' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 30,
            'category_keys' => ['getting-started', 'game-systems'],
            'slugs' => ['en' => 'creating-a-character', 'pl' => 'tworzenie-postaci'],
        ],
        'first-login' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 40,
            'category_keys' => ['getting-started'],
            'slugs' => ['en' => 'first-login', 'pl' => 'pierwsze-logowanie'],
        ],
        'server-information' => [
            'content_type' => 'reference',
            'featured' => true,
            'sort_order' => 50,
            'category_keys' => ['server-information'],
            'slugs' => ['en' => 'server-information', 'pl' => 'informacje-o-serwerze'],
        ],
        'server-rates' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 60,
            'category_keys' => ['server-information'],
            'slugs' => ['en' => 'server-rates', 'pl' => 'tempo-serwera'],
        ],
        'vocations' => [
            'content_type' => 'system',
            'featured' => false,
            'sort_order' => 70,
            'category_keys' => ['game-systems'],
            'slugs' => ['en' => 'vocations', 'pl' => 'profesje'],
        ],
        'pvp-and-game-rules' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 80,
            'category_keys' => ['server-information', 'support'],
            'slugs' => ['en' => 'pvp-and-game-rules', 'pl' => 'pvp-i-zasady-gry'],
        ],
        'account-security-and-mfa' => [
            'content_type' => 'guide',
            'featured' => true,
            'sort_order' => 90,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'account-security-and-mfa', 'pl' => 'bezpieczenstwo-konta-i-mfa'],
        ],
        'frequently-asked-questions' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 100,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'frequently-asked-questions', 'pl' => 'najczesciej-zadawane-pytania'],
        ],
        'known-issues' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 110,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'known-issues', 'pl' => 'znane-problemy'],
        ],
        'discord-and-support' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 120,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'discord-and-support', 'pl' => 'discord-i-wsparcie'],
        ],
        'report-a-bug' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 130,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'report-a-bug', 'pl' => 'zglos-blad'],
        ],
    ];

    /**
     * First-party non-Wiki paths explicitly referenced by reviewed launch content.
     * Localized Wiki article links are validated against ARTICLES instead.
     *
     * @var list<string>
     */
    public const INTERNAL_PATHS = [
        '/',
        '/account',
        '/account/characters/create',
        '/announcements',
        '/download',
        '/mfa',
        '/register',
        '/rules',
        '/server-information',
        '/support',
        '/support/report-a-bug',
    ];

    public const EXPECTED_EDITORIAL_MEDIA_TOKENS = 0;

    public const MEDIA_FALLBACK_POLICY = 'editorial-media-optional-with-explicit-unavailable-fallback';
}
