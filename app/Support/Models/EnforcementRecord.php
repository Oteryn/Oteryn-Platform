<?php

namespace App\Support\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $public_id
 * @property int $identity_id
 * @property string $category
 * @property string $status
 * @property string $public_reason
 * @property string|null $moderator_notes
 * @property Carbon $effective_at
 * @property Carbon|null $expires_at
 * @property Carbon|null $acknowledged_at
 * @property string $appeal_status
 * @property string|null $appeal_message
 * @property string|null $appeal_outcome
 * @property int|null $created_by
 * @property int|null $updated_by
 * @property int $lock_version
 */
final class EnforcementRecord extends Model
{
    public const CATEGORY_WARNING = 'warning';
    public const CATEGORY_RESTRICTION = 'restriction';
    public const CATEGORY_SUSPENSION = 'suspension';

    public const STATUS_ACTIVE = 'active';
    public const STATUS_EXPIRED = 'expired';
    public const STATUS_REVOKED = 'revoked';

    public const APPEAL_NONE = 'none';
    public const APPEAL_REQUESTED = 'requested';
    public const APPEAL_REVIEWING = 'reviewing';
    public const APPEAL_ACCEPTED = 'accepted';
    public const APPEAL_REJECTED = 'rejected';

    /** @var list<string> */
    protected $fillable = [
        'public_id',
        'identity_id',
        'category',
        'status',
        'public_reason',
        'moderator_notes',
        'effective_at',
        'expires_at',
        'acknowledged_at',
        'appeal_status',
        'appeal_message',
        'appeal_outcome',
        'created_by',
        'updated_by',
        'lock_version',
    ];

    public function getRouteKeyName(): string
    {
        return 'public_id';
    }

    /** @return list<string> */
    public static function categories(): array
    {
        return [self::CATEGORY_WARNING, self::CATEGORY_RESTRICTION, self::CATEGORY_SUSPENSION];
    }

    /** @return list<string> */
    public static function statuses(): array
    {
        return [self::STATUS_ACTIVE, self::STATUS_EXPIRED, self::STATUS_REVOKED];
    }

    /** @return list<string> */
    public static function appealStatuses(): array
    {
        return [
            self::APPEAL_NONE,
            self::APPEAL_REQUESTED,
            self::APPEAL_REVIEWING,
            self::APPEAL_ACCEPTED,
            self::APPEAL_REJECTED,
        ];
    }

    /** @return BelongsTo<Identity, $this> */
    public function identity(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'identity_id');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'created_by' => 'integer',
            'updated_by' => 'integer',
            'lock_version' => 'integer',
            'effective_at' => 'datetime',
            'expires_at' => 'datetime',
            'acknowledged_at' => 'datetime',
        ];
    }
}
