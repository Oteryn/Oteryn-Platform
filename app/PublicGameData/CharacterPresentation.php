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

    public function vocationName(int $vocation): string
    {
        $key = self::VOCATION_KEYS[$vocation] ?? 'unknown';

        return (string) __('game.vocations.'.$key, ['id' => $vocation]);
    }
}
