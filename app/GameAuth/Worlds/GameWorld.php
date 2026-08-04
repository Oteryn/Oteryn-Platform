<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;

/**
 * @property int $id
 * @property string $slug
 * @property string $name
 * @property string $region
 * @property GameWorldStatus $status
 * @property bool $login_enabled
 * @property string $game_host
 * @property int $game_port
 * @property int $gameplay_policy_revision
 */
final class GameWorld extends Model
{
    protected $table = 'game_worlds';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'slug',
        'name',
        'region',
        'status',
        'login_enabled',
        'game_host',
        'game_port',
        'gameplay_policy_revision',
    ];

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'status' => GameWorldStatus::class,
            'login_enabled' => 'boolean',
            'game_port' => 'integer',
            'gameplay_policy_revision' => 'integer',
        ];
    }

    /**
     * @return HasMany<GameWorldProtocolCandidate, $this>
     */
    public function protocolCandidates(): HasMany
    {
        return $this->hasMany(GameWorldProtocolCandidate::class, 'game_world_id');
    }
}
