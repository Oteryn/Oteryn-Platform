<?php

namespace App\Identity\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property string $id
 * @property int $identity_id
 * @property int $generation
 * @property string|null $user_agent
 * @property string|null $ip_hash
 * @property Carbon $issued_at
 * @property Carbon $last_seen_at
 * @property Carbon $expires_at
 * @property Carbon|null $revoked_at
 */
final class IdentityWebSession extends Model
{
    protected $table = 'identity_web_sessions';

    protected $keyType = 'string';

    public $incrementing = false;

    /** @var list<string> */
    protected $fillable = [
        'id',
        'identity_id',
        'generation',
        'user_agent',
        'ip_hash',
        'issued_at',
        'last_seen_at',
        'expires_at',
        'revoked_at',
    ];

    /** @return BelongsTo<Identity, $this> */
    public function identity(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'identity_id');
    }

    public function isActive(): bool
    {
        return $this->revoked_at === null && $this->expires_at->isFuture();
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'generation' => 'integer',
            'issued_at' => 'datetime',
            'last_seen_at' => 'datetime',
            'expires_at' => 'datetime',
            'revoked_at' => 'datetime',
        ];
    }
}
