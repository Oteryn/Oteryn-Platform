<?php

namespace App\Identity\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $identity_id
 * @property string $key_hash
 * @property Carbon $generated_at
 * @property Carbon|null $used_at
 * @property Carbon|null $revoked_at
 */
final class IdentityRecoveryKey extends Model
{
    protected $table = 'identity_recovery_keys';

    protected $primaryKey = 'identity_id';

    public $incrementing = false;

    /** @var list<string> */
    protected $fillable = [
        'identity_id',
        'key_hash',
        'generated_at',
        'used_at',
        'revoked_at',
    ];

    /** @return BelongsTo<Identity, $this> */
    public function identity(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'identity_id');
    }

    public function isActive(): bool
    {
        return $this->used_at === null && $this->revoked_at === null;
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'generated_at' => 'datetime',
            'used_at' => 'datetime',
            'revoked_at' => 'datetime',
        ];
    }
}
