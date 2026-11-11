<?php

namespace App\Accounts\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $identity_id
 * @property int|null $canary_account_id
 * @property string $provisioning_name
 * @property int $canary_creation_epoch
 * @property string $status
 * @property string|null $last_failure_code
 * @property Carbon|null $last_attempt_at
 * @property Carbon|null $ready_at
 */
final class IdentityCanaryAccount extends Model
{
    public const STATUS_PENDING = 'pending';

    public const STATUS_READY = 'ready';

    public const STATUS_CONFLICT = 'conflict';

    protected $table = 'identity_canary_accounts';

    protected $primaryKey = 'identity_id';

    public $incrementing = false;

    protected $keyType = 'int';

    /**
     * @var list<string>
     */
    protected $fillable = [
        'identity_id',
        'canary_account_id',
        'provisioning_name',
        'canary_creation_epoch',
        'status',
        'last_failure_code',
        'last_attempt_at',
        'ready_at',
    ];

    public function isReady(): bool
    {
        return $this->status === self::STATUS_READY && $this->canary_account_id !== null;
    }

    public function isConflict(): bool
    {
        return $this->status === self::STATUS_CONFLICT;
    }

    /**
     * @return array<string, string>
     */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'canary_account_id' => 'integer',
            'canary_creation_epoch' => 'integer',
            'last_attempt_at' => 'datetime',
            'ready_at' => 'datetime',
        ];
    }
}
