<?php

namespace App\GameCatalog\Security;

final class GameCatalogPermission
{
    public const ACCESS = 'game_catalog.access';
    public const SNAPSHOTS_VIEW = 'game_catalog.snapshots.view';
    public const SNAPSHOTS_IMPORT = 'game_catalog.snapshots.import';
    public const SNAPSHOTS_ACTIVATE = 'game_catalog.snapshots.activate';
    public const PROFILES_MANAGE = 'game_catalog.profiles.manage';
    public const TRANSLATIONS_MANAGE = 'game_catalog.translations.manage';
    public const OVERRIDES_MANAGE = 'game_catalog.overrides.manage';

    /** @return array<string, string> */
    public static function definitions(): array
    {
        return [
            self::ACCESS => 'Access Game Catalog administration',
            self::SNAPSHOTS_VIEW => 'View Game Catalog snapshots and findings',
            self::SNAPSHOTS_IMPORT => 'Import Game Catalog snapshots through controlled operator paths',
            self::SNAPSHOTS_ACTIVATE => 'Activate and roll back Game Catalog snapshots',
            self::PROFILES_MANAGE => 'Manage Game Catalog content profiles',
            self::TRANSLATIONS_MANAGE => 'Manage Game Catalog translations',
            self::OVERRIDES_MANAGE => 'Manage reviewed Game Catalog overrides',
        ];
    }

    private function __construct() {}
}
