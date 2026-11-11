<?php

namespace App\Wiki\Content;

final class WikiExpectedContentInventory
{
    public const VERSION = '2026-08-10.2';

    public const CATALOG_VERSION = '2026-07-26.1';

    /**
     * Git blob SHA of the exact reviewed WikiLaunchContentCatalog.php source.
     * This independently pins every localized title, summary, Markdown body,
     * category name/description and source-reference path in the reviewed corpus.
     */
    public const CATALOG_SOURCE_GIT_BLOB_SHA = '07ff3324a4530958f9f4e164c5f7a2a399a1bb8b';

    public const EFFECTIVE_FROM = '2026-08-10';

    /** @var list<string> */
    public const LOCALES = ['en', 'pl'];

    /**
     * The authoritative expected category set for the reviewed launch-content scope.
     * Exact localized category copy is additionally pinned by CATALOG_SOURCE_GIT_BLOB_SHA.
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
     * Exact localized article copy is additionally pinned by CATALOG_SOURCE_GIT_BLOB_SHA.
     * Provenance paths are repeated here independently so an unrelated existing file
     * cannot silently replace a reviewed source reference.
     *
     * @var array<string, array{
     *     content_type: string,
     *     featured: bool,
     *     sort_order: int,
     *     category_keys: list<string>,
     *     slugs: array{en: string, pl: string},
     *     source_references: list<string>
     * }>
     */
    public const ARTICLES = [
        'download-and-installation' => [
            'content_type' => 'guide',
            'featured' => true,
            'sort_order' => 10,
            'category_keys' => ['getting-started'],
            'slugs' => ['en' => 'download-and-installation', 'pl' => 'pobieranie-i-instalacja'],
            'source_references' => [
                'app/Downloads/PublicDownloadCenterQuery.php',
                'app/Downloads/Actions/PublishClientRelease.php',
                'resources/views/downloads/index.blade.php',
            ],
        ],
        'creating-an-account' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 20,
            'category_keys' => ['getting-started'],
            'slugs' => ['en' => 'creating-an-account', 'pl' => 'tworzenie-konta'],
            'source_references' => [
                'app/Http/Controllers/Identity/RegistrationController.php',
                'app/Accounts/ReadModels/AccountOverviewReadModel.php',
                'docs/architecture/adr/0004-authoritative-platform-account-ownership.md',
            ],
        ],
        'creating-a-character' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 30,
            'category_keys' => ['getting-started', 'game-systems'],
            'slugs' => ['en' => 'creating-a-character', 'pl' => 'tworzenie-postaci'],
            'source_references' => [
                'docs/architecture/adr/0005-character-creation-product-policy.md',
                'app/Characters/Actions/CreateCharacter.php',
                'resources/views/characters/create.blade.php',
            ],
        ],
        'first-login' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 40,
            'category_keys' => ['getting-started'],
            'slugs' => ['en' => 'first-login', 'pl' => 'pierwsze-logowanie'],
            'source_references' => [
                'docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md',
                'docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md',
                'docs/agents/PROJECT_STATE.md',
            ],
        ],
        'server-information' => [
            'content_type' => 'reference',
            'featured' => true,
            'sort_order' => 50,
            'category_keys' => ['server-information'],
            'slugs' => ['en' => 'server-information', 'pl' => 'informacje-o-serwerze'],
            'source_references' => [
                'app/PublicPortal/HomePageQuery.php',
                'resources/views/home.blade.php',
                'app/Cms/Editorial/EditorialPageKey.php',
            ],
        ],
        'server-rates' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 60,
            'category_keys' => ['server-information'],
            'slugs' => ['en' => 'server-rates', 'pl' => 'tempo-serwera'],
            'source_references' => [
                'docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md',
                'app/Cms/Editorial/EditorialPageKey.php',
            ],
        ],
        'vocations' => [
            'content_type' => 'system',
            'featured' => false,
            'sort_order' => 70,
            'category_keys' => ['game-systems'],
            'slugs' => ['en' => 'vocations', 'pl' => 'profesje'],
            'source_references' => [
                'docs/architecture/adr/0005-character-creation-product-policy.md',
                'app/Characters/Actions/CreateCharacter.php',
                'resources/views/characters/create.blade.php',
            ],
        ],
        'pvp-and-game-rules' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 80,
            'category_keys' => ['server-information', 'support'],
            'slugs' => ['en' => 'pvp-and-game-rules', 'pl' => 'pvp-i-zasady-gry'],
            'source_references' => [
                'app/PublicPortal/HomePageQuery.php',
                'app/Cms/Editorial/EditorialPageKey.php',
                'docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md',
            ],
        ],
        'account-security-and-mfa' => [
            'content_type' => 'guide',
            'featured' => true,
            'sort_order' => 90,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'account-security-and-mfa', 'pl' => 'bezpieczenstwo-konta-i-mfa'],
            'source_references' => [
                'docs/architecture/SECURITY_ARCHITECTURE.md',
                'app/Identity/Mfa/ConfirmIdentityMfaEnrollment.php',
                'app/Identity/Mfa/MfaRecoveryCodes.php',
            ],
        ],
        'frequently-asked-questions' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 100,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'frequently-asked-questions', 'pl' => 'najczesciej-zadawane-pytania'],
            'source_references' => [
                'app/Accounts/ReadModels/AccountOverviewReadModel.php',
                'app/Characters/Actions/CreateCharacter.php',
                'docs/architecture/adr/0013-wiki-administration.md',
            ],
        ],
        'known-issues' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 110,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'known-issues', 'pl' => 'znane-problemy'],
            'source_references' => [
                'app/Cms/Editorial/EditorialPageKey.php',
                'docs/architecture/SECURITY_ARCHITECTURE.md',
                'resources/views/downloads/index.blade.php',
            ],
        ],
        'discord-and-support' => [
            'content_type' => 'reference',
            'featured' => false,
            'sort_order' => 120,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'discord-and-support', 'pl' => 'discord-i-wsparcie'],
            'source_references' => [
                'app/Cms/Editorial/EditorialPageKey.php',
                'routes/modules/support.php',
                'docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md',
            ],
        ],
        'report-a-bug' => [
            'content_type' => 'guide',
            'featured' => false,
            'sort_order' => 130,
            'category_keys' => ['support'],
            'slugs' => ['en' => 'report-a-bug', 'pl' => 'zglos-blad'],
            'source_references' => [
                'app/Cms/Editorial/EditorialPageKey.php',
                'routes/modules/support.php',
                'docs/architecture/SECURITY_ARCHITECTURE.md',
            ],
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
