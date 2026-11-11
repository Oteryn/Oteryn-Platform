<?php

namespace App\Support\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $public_id
 * @property int $reporter_identity_id
 * @property string $request_key
 * @property string $report_type
 * @property string $category
 * @property string $target_reference
 * @property string|null $evidence_summary
 * @property string $status
 * @property string|null $public_outcome
 * @property string|null $moderator_notes
 * @property int|null $assigned_to
 * @property int $lock_version
 * @property Carbon|null $processed_at
 */
final class PlayerReport extends Model
{
    public const TYPE_PLAYER = 'player';

    public const TYPE_CONTENT = 'content';

    public const TYPE_GUILD = 'guild';

    public const STATUS_SUBMITTED = 'submitted';

    public const STATUS_REVIEWING = 'reviewing';

    public const STATUS_ACTIONED = 'actioned';

    public const STATUS_REJECTED = 'rejected';

    public const STATUS_CLOSED = 'closed';

    /** @var list<string> */
    protected $fillable = [
        'public_id',
        'reporter_identity_id',
        'request_key',
        'report_type',
        'category',
        'target_reference',
        'evidence_summary',
        'status',
        'public_outcome',
        'moderator_notes',
        'assigned_to',
        'lock_version',
        'processed_at',
    ];

    public function getRouteKeyName(): string
    {
        return 'public_id';
    }

    /** @return list<string> */
    public static function types(): array
    {
        return [self::TYPE_PLAYER, self::TYPE_CONTENT, self::TYPE_GUILD];
    }

    /** @return list<string> */
    public static function statuses(): array
    {
        return [
            self::STATUS_SUBMITTED,
            self::STATUS_REVIEWING,
            self::STATUS_ACTIONED,
            self::STATUS_REJECTED,
            self::STATUS_CLOSED,
        ];
    }

    public function isPending(): bool
    {
        return in_array($this->status, [self::STATUS_SUBMITTED, self::STATUS_REVIEWING], true);
    }

    /** @return BelongsTo<Identity, $this> */
    public function reporter(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'reporter_identity_id');
    }

    /** @return BelongsTo<Identity, $this> */
    public function assignee(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'assigned_to');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'reporter_identity_id' => 'integer',
            'assigned_to' => 'integer',
            'lock_version' => 'integer',
            'processed_at' => 'datetime',
        ];
    }
}
