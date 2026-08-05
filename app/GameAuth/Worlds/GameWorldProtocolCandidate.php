<?php

namespace App\GameAuth\Worlds;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

/**
 * @property int $id
 * @property int $game_world_id
 * @property int $channel_id
 * @property int $sort_order
 * @property string $family
 * @property string $nativeProtocolVersion
 * @property string $transport
 * @property int $schema_revision
 * @property string $schema_sha256
 * @property list<string> $required_capabilities
 * @property list<string> $optional_capabilities
 * @property string $endpoint_id
 * @property string $game_host
 * @property int $game_port
 * @property string $tls_server_name
 * @property bool $enabled
 */
final class GameWorldProtocolCandidate extends Model
{
    protected $table = 'game_world_protocol_candidates';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'game_world_id',
        'channel_id',
        'sort_order',
        'family',
        'native_protocol_version',
        'transport',
        'schema_revision',
        'schema_sha256',
        'required_capabilities',
        'optional_capabilities',
        'endpoint_id',
        'game_host',
        'game_port',
        'tls_server_name',
        'enabled',
    ];

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'channel_id' => 'integer',
            'sort_order' => 'integer',
            'native_protocol_version' => 'integer',
            'schema_revision' => 'integer',
            'required_capabilities' => 'array',
            'optional_capabilities' => 'array',
            'game_port' => 'integer',
            'enabled' => 'boolean',
        ];
    }

    /**
     * @return BelongsTo<GameWorld, $this>
     */
    public function world(): BelongsTo
    {
        return $this->belongsTo(GameWorld::class, 'game_world_id');
    }
}
