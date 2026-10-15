<?php

namespace App\PublicGameData;

use InvalidArgumentException;

final class CommunityDataPolicy
{
    /** @var array<string, literal-string> */
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

    /**
     * Public Projections v1 §4.1 category ids (HIGHSCORES-0 §3). `level` and `experience` share
     * category 1: the published value is total experience and the row carries the level.
     */
    private const HIGHSCORE_CATEGORY_IDS = [
        'level' => 1,
        'experience' => 1,
        'magic' => 2,
        'fist' => 3,
        'club' => 4,
        'sword' => 5,
        'axe' => 6,
        'distance' => 7,
        'shielding' => 8,
        'fishing' => 9,
    ];

    /** Base vocation id to the published filter 1..5 (0 = all), promoted vocations count as their base. */
    private const HIGHSCORE_VOCATION_FILTERS = [
        1 => 1, 5 => 1,
        2 => 2, 6 => 2,
        3 => 3, 7 => 3,
        4 => 4, 8 => 4,
        9 => 5, 10 => 5,
    ];

    /** @return list<string> */
    public static function highscoreCategories(): array
    {
        return array_keys(self::HIGHSCORE_COLUMNS);
    }

    /** @return literal-string */
    public static function highscoreColumn(string $category): string
    {
        $column = self::HIGHSCORE_COLUMNS[$category] ?? null;

        if ($column === null) {
            throw new InvalidArgumentException('Unsupported highscore category.');
        }

        return $column;
    }

    public static function highscoreCategoryId(string $category): int
    {
        return self::HIGHSCORE_CATEGORY_IDS[$category]
            ?? throw new InvalidArgumentException('Unsupported highscore category.');
    }

    /** The published filter for a vocation; 0 (all) for an id with no base list. */
    public static function highscoreVocationFilter(int $vocation): int
    {
        return self::HIGHSCORE_VOCATION_FILTERS[$vocation] ?? 0;
    }

    /** @return list<int> */
    public static function highscoreVocationIds(): array
    {
        return [1, 2, 3, 4, 9];
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
