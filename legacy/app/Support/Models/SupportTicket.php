<?php

namespace App\Support\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $public_id
 * @property int $identity_id
 * @property string $request_key
 * @property string $category
 * @property string $subject
 * @property string $status
 * @property int $lock_version
 * @property Carbon $last_message_at
 * @property Carbon|null $closed_at
 */
final class SupportTicket extends Model
{
    public const STATUS_OPEN = 'open';

    public const STATUS_WAITING_USER = 'waiting_user';

    public const STATUS_WAITING_STAFF = 'waiting_staff';

    public const STATUS_RESOLVED = 'resolved';

    public const STATUS_CLOSED = 'closed';

    public const CATEGORY_ACCOUNT = 'account';

    public const CATEGORY_CHARACTER = 'character';

    public const CATEGORY_TECHNICAL = 'technical';

    public const CATEGORY_PAYMENT = 'payment';

    public const CATEGORY_OTHER = 'other';

    /** @var list<string> */
    protected $fillable = [
        'public_id',
        'identity_id',
        'request_key',
        'category',
        'subject',
        'status',
        'lock_version',
        'last_message_at',
        'closed_at',
    ];

    public function getRouteKeyName(): string
    {
        return 'public_id';
    }

    /** @return list<string> */
    public static function statuses(): array
    {
        return [
            self::STATUS_OPEN,
            self::STATUS_WAITING_USER,
            self::STATUS_WAITING_STAFF,
            self::STATUS_RESOLVED,
            self::STATUS_CLOSED,
        ];
    }

    /** @return list<string> */
    public static function categories(): array
    {
        return [
            self::CATEGORY_ACCOUNT,
            self::CATEGORY_CHARACTER,
            self::CATEGORY_TECHNICAL,
            self::CATEGORY_PAYMENT,
            self::CATEGORY_OTHER,
        ];
    }

    public function allowsUserReply(): bool
    {
        return in_array($this->status, [self::STATUS_OPEN, self::STATUS_WAITING_USER], true);
    }

    /** @return BelongsTo<Identity, $this> */
    public function identity(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'identity_id');
    }

    /** @return HasMany<SupportTicketMessage, $this> */
    public function messages(): HasMany
    {
        return $this->hasMany(SupportTicketMessage::class, 'support_ticket_id');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'lock_version' => 'integer',
            'last_message_at' => 'datetime',
            'closed_at' => 'datetime',
        ];
    }
}
