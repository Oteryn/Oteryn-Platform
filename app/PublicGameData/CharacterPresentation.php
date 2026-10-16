<?php

namespace App\PublicGameData;

final class CharacterPresentation
{
    /** @var array<int, string> */
    private const VOCATION_KEYS = [
        0 => 'none',
        1 => 'sorcerer',
        2 => 'druid',
        3 => 'paladin',
        4 => 'knight',
        5 => 'master_sorcerer',
        6 => 'elder_druid',
        7 => 'royal_paladin',
        8 => 'elite_knight',
        9 => 'monk',
        10 => 'exalted_monk',
    ];

    /** @var array<int, string> */
    private const PUBLISHED_VOCATION_KEYS = [
        1 => 'druid',
        2 => 'knight',
        3 => 'monk',
        4 => 'paladin',
        5 => 'sorcerer',
        6 => 'none',
    ];

    public function vocationName(int $vocation): string
    {
        $key = self::VOCATION_KEYS[$vocation] ?? 'unknown';

        return (string) __('game.vocations.'.$key, ['id' => $vocation]);
    }

    /**
     * Name of a vocation id published by Game Public Projections v1, which carries Game's
     * reference base-vocation ids (not the legacy ids of {@see vocationName()}).
     */
    public function publishedVocationName(int $vocation): string
    {
        $key = self::PUBLISHED_VOCATION_KEYS[$vocation] ?? 'unknown';

        return (string) __('game.vocations.'.$key, ['id' => $vocation]);
    }
}
