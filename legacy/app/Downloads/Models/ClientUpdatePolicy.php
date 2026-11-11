<?php

namespace App\Downloads\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $operation_id
 * @property string $channel
 * @property int $revision
 * @property int $current_release_id
 * @property int $current_release_sequence
 * @property int $minimum_supported_release_sequence
 * @property string $update_mode
 * @property array<int, array<string, int|string>> $artifact_targets
 * @property list<string> $revoked_release_ids
 * @property list<string> $revoked_artifact_targets
 * @property string $rollback_authorization
 * @property string $policy_target_path
 * @property string $policy_document_sha256
 * @property int $policy_document_length
 * @property Carbon $approved_at
 * @property-read ClientRelease $currentRelease
 */
final class ClientUpdatePolicy extends Model
{
    /** @var list<string> */
    protected $fillable = [
        'operation_id',
        'channel',
        'revision',
        'current_release_id',
        'current_release_sequence',
        'minimum_supported_release_sequence',
        'update_mode',
        'artifact_targets',
        'revoked_release_ids',
        'revoked_artifact_targets',
        'rollback_authorization',
        'policy_target_path',
        'policy_document_sha256',
        'policy_document_length',
        'approved_at',
    ];

    /** @return BelongsTo<ClientRelease, $this> */
    public function currentRelease(): BelongsTo
    {
        return $this->belongsTo(ClientRelease::class, 'current_release_id');
    }

    /** @return HasMany<ClientUpdateGeneration, $this> */
    public function generations(): HasMany
    {
        return $this->hasMany(ClientUpdateGeneration::class, 'policy_id');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'revision' => 'integer',
            'current_release_id' => 'integer',
            'current_release_sequence' => 'integer',
            'minimum_supported_release_sequence' => 'integer',
            'artifact_targets' => 'array',
            'revoked_release_ids' => 'array',
            'revoked_artifact_targets' => 'array',
            'policy_document_length' => 'integer',
            'approved_at' => 'datetime',
        ];
    }
}
