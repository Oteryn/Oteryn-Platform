<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use LogicException;

/**
 * @property int $id
 * @property int $game_world_id
 * @property string $channel_id
 * @property string $channel_key
 */
final class GameChannel extends Model
{
    protected $table = 'game_channels';

    /** @var list<string> */
    protected $guarded = ['*'];

    protected static function booted(): void
    {
        self::creating(function (): void {
            throw new LogicException('Canonical Channel issuance belongs to the native topology Registry.');
        });
        self::updating(function (self $channel): void {
            if ($channel->isDirty(['game_world_id', 'channel_id', 'channel_key'])) {
                throw new LogicException('An issued Channel identity and World binding are immutable.');
            }
        });
        self::deleting(function (): void {
            throw new LogicException('Issued native topology identities must be retained.');
        });
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return ['game_world_id' => 'integer'];
    }

    /** @return BelongsTo<GameWorld, $this> */
    public function world(): BelongsTo
    {
        return $this->belongsTo(GameWorld::class, 'game_world_id');
    }
}
