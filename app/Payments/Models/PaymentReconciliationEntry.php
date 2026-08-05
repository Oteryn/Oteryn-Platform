<?php

namespace App\Payments\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int|null $payment_order_id
 * @property int|null $payment_provider_event_id
 * @property string $issue_type
 * @property string $state
 * @property array<string, bool|int|string|null>|null $metadata
 * @property Carbon $created_at
 * @property Carbon|null $resolved_at
 */
final class PaymentReconciliationEntry extends Model
{
    public const UPDATED_AT = null;

    public const STATE_OPEN = 'open';

    public const STATE_RESOLVED = 'resolved';

    /** @var list<string> */
    protected $fillable = [
        'payment_order_id',
        'payment_provider_event_id',
        'issue_type',
        'state',
        'metadata',
        'created_at',
        'resolved_at',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'payment_order_id' => 'integer',
            'payment_provider_event_id' => 'integer',
            'metadata' => 'array',
            'created_at' => 'datetime',
            'resolved_at' => 'datetime',
        ];
    }
}
