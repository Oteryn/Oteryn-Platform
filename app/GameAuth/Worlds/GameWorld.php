<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;
use LogicException;

/**
 * @property int $id
 * @property string|null $world_id
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

    protected static function booted(): void
    {
        self::creating(function (self $world): void {
            if ($world->getAttribute('world_id') !== null) {
                throw new LogicException('Canonical WorldId issuance belongs to the native topology Registry.');
            }
        });
        self::updating(function (self $world): void {
            if ($world->isDirty('world_id')) {
                throw new LogicException('An issued canonical WorldId cannot be replaced or cleared.');
            }
        });
        self::deleting(function (self $world): void {
            if (self::query()->whereKey($world->id)->whereNotNull('world_id')->exists()
                || $world->channels()->exists()) {
                throw new LogicException('Issued native topology identities must be retained.');
            }
        });
    }

    /**
     * @return HasMany<GameChannel, $this>
     */
    public function channels(): HasMany
    {
        return $this->hasMany(GameChannel::class, 'game_world_id');
    }

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
