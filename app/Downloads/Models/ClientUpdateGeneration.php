<?php

namespace App\Downloads\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $generation_id
 * @property int $policy_id
 * @property string $channel
 * @property int $root_version
 * @property int $targets_version
 * @property int $snapshot_version
 * @property int $timestamp_version
 * @property Carbon $metadata_expires_at
 * @property string $metadata_set_sha256
 * @property string $policy_target_path
 * @property string $policy_target_sha256
 * @property int $policy_target_length
 * @property list<array{platform: string, architecture: string, target_path: string, length: int, sha256: string}> $targets
 * @property Carbon $reconciled_at
 * @property Carbon|null $activated_at
 * @property Carbon|null $superseded_at
 * @property-read ClientUpdatePolicy $policy
 */
final class ClientUpdateGeneration extends Model
{
    /** @var list<string> */
    protected $fillable = [
        'generation_id',
        'policy_id',
        'channel',
        'root_version',
        'targets_version',
        'snapshot_version',
        'timestamp_version',
        'metadata_expires_at',
        'metadata_set_sha256',
        'policy_target_path',
        'policy_target_sha256',
        'policy_target_length',
        'targets',
        'reconciled_at',
        'activated_at',
        'superseded_at',
    ];

    /** @return BelongsTo<ClientUpdatePolicy, $this> */
    public function policy(): BelongsTo
    {
        return $this->belongsTo(ClientUpdatePolicy::class, 'policy_id');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'policy_id' => 'integer',
            'root_version' => 'integer',
            'targets_version' => 'integer',
            'snapshot_version' => 'integer',
            'timestamp_version' => 'integer',
            'metadata_expires_at' => 'datetime',
            'policy_target_length' => 'integer',
            'targets' => 'array',
            'reconciled_at' => 'datetime',
            'activated_at' => 'datetime',
            'superseded_at' => 'datetime',
        ];
    }
}
