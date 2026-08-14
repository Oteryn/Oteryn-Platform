<?php

namespace App\PlayerCompanion\Models;

use Illuminate\Database\Eloquent\Model;

/**
 * Owner-private normalized Hunt Session Analyzer result.
 *
 * Raw submitted session text is intentionally never persisted by this model.
 *
 * @property array{schema_version:string,game_profile:string,ruleset:string,catalog_snapshot:string,world:string,season:string,effective_scope:string} $applicability
 * @property list<array{from:string,to:string,amount:int}> $settlements
 */
final class SessionAnalysis extends Model
{
    protected $table = 'player_companion_session_analyses';

    /** @var list<string> */
    protected $fillable = [
        'identity_id',
        'label',
        'source_format',
        'parser_version',
        'formula_version',
        'applicability',
        'session_seconds',
        'experience_gain',
        'loot_value',
        'supplies_value',
        'balance_value',
        'damage',
        'healing',
        'experience_per_hour',
        'profit_per_hour',
        'participant_count',
        'participants',
        'settlements',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'applicability' => 'array',
            'session_seconds' => 'integer',
            'experience_gain' => 'integer',
            'loot_value' => 'integer',
            'supplies_value' => 'integer',
            'balance_value' => 'integer',
            'damage' => 'integer',
            'healing' => 'integer',
            'experience_per_hour' => 'integer',
            'profit_per_hour' => 'integer',
            'participant_count' => 'integer',
            'participants' => 'array',
            'settlements' => 'array',
        ];
    }
}
