<?php

namespace App\Support\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

final class SupportNotificationDelivery extends Model
{
    public const STATUS_PENDING = 'pending';

    public const STATUS_SENT = 'sent';

    public const STATUS_FAILED = 'failed';

    /** @var list<string> */
    protected $fillable = [
        'identity_id',
        'event_key',
        'related_type',
        'related_id',
        'locale',
        'status',
        'attempts',
        'last_error_code',
        'sent_at',
        'failed_at',
    ];

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
            'attempts' => 'integer',
            'sent_at' => 'datetime',
            'failed_at' => 'datetime',
        ];
    }
}
