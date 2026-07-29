<?php

namespace App\PublicGameData;

use InvalidArgumentException;

final class CommunityDataPolicy
{
    /** @var array<string, string> */
    private const HIGHSCORE_COLUMNS = [
        'level' => 'level',
        'experience' => 'experience',
        'magic' => 'maglevel',
        'fist' => 'skill_fist',
        'club' => 'skill_club',
        'sword' => 'skill_sword',
        'axe' => 'skill_axe',
        'distance' => 'skill_dist',
        'shielding' => 'skill_shielding',
        'fishing' => 'skill_fishing',
    ];

    /** @return list<string> */
    public static function highscoreCategories(): array
    {
        return array_keys(self::HIGHSCORE_COLUMNS);
    }

    public static function highscoreColumn(string $category): string
    {
        $column = self::HIGHSCORE_COLUMNS[$category] ?? null;

        if ($column === null) {
            throw new InvalidArgumentException('Unsupported highscore category.');
        }

        return $column;
    }

    /** @return list<int> */
    public static function vocationIds(): array
    {
        return range(0, 10);
    }

    public static function guildSearchLimit(): int
    {
        return 80;
    }

    public static function profileDeathLimit(): int
    {
        return 10;
    }

    public static function profileRelatedCharacterLimit(): int
    {
        return 20;
    }

    public static function profileRecentKillLimit(): int
    {
        return 10;
    }
}
