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

    /**
     * The published highscores filter of a base vocation. Game numbers filters 1..5 by its
     * reference base-vocation ids (Druid 1, Knight 2, Monk 3, Paladin 4, Sorcerer 5); 0 is all.
     */
    public static function highscoreVocationFilter(int $vocation): int
    {
        return $vocation >= 1 && $vocation <= 5 ? $vocation : 0;
    }

    /** @return list<int> the base vocations a highscores page can filter by */
    public static function highscoreVocationIds(): array
    {
        return [1, 2, 3, 4, 5];
    }

    /** @return list<int> accepted `vocation` query values; 0 means all */
    public static function vocationIds(): array
    {
        return [0, 1, 2, 3, 4, 5];
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
