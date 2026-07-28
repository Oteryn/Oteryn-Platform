<?php

namespace App\Identity\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property string $id
 * @property int $identity_id
 * @property string $old_email
 * @property string $new_email
 * @property string $verification_token_hash
 * @property string $recovery_token_hash
 * @property Carbon $requested_at
 * @property Carbon $expires_at
 * @property Carbon|null $confirmed_at
 * @property Carbon|null $recoverable_until
 * @property Carbon|null $recovered_at
 * @property Carbon|null $cancelled_at
 */
final class IdentityEmailChangeRequest extends Model
{
    protected $table = 'identity_email_change_requests';

    protected $keyType = 'string';

    public $incrementing = false;

    /** @var list<string> */
    protected $fillable = [
        'id',
        'identity_id',
        'old_email',
        'new_email',
        'verification_token_hash',
        'recovery_token_hash',
        'requested_at',
        'expires_at',
        'confirmed_at',
        'recoverable_until',
        'recovered_at',
        'cancelled_at',
    ];

    /** @return BelongsTo<Identity, $this> */
    public function identity(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'identity_id');
    }

    public function isPending(): bool
    {
        return $this->confirmed_at === null
            && $this->recovered_at === null
            && $this->cancelled_at === null
            && $this->expires_at->isFuture();
    }

    public function isRecoverable(): bool
    {
        return $this->confirmed_at !== null
            && $this->recoverable_until?->isFuture() === true
            && $this->recovered_at === null
            && $this->cancelled_at === null;
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'requested_at' => 'datetime',
            'expires_at' => 'datetime',
            'confirmed_at' => 'datetime',
            'recoverable_until' => 'datetime',
            'recovered_at' => 'datetime',
            'cancelled_at' => 'datetime',
        ];
    }
}
