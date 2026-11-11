<?php

namespace App\Payments\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $payment_order_id
 * @property int|null $payment_provider_event_id
 * @property string|null $from_status
 * @property string $to_status
 * @property string $reason
 * @property int|null $verified_refund_amount_minor
 * @property int|null $refunded_total_minor
 * @property int $version
 * @property Carbon $created_at
 */
final class PaymentOrderTransition extends Model
{
    public const UPDATED_AT = null;

    /** @var list<string> */
    protected $fillable = [
        'payment_order_id',
        'payment_provider_event_id',
        'from_status',
        'to_status',
        'reason',
        'verified_refund_amount_minor',
        'refunded_total_minor',
        'version',
        'created_at',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'payment_order_id' => 'integer',
            'payment_provider_event_id' => 'integer',
            'verified_refund_amount_minor' => 'integer',
            'refunded_total_minor' => 'integer',
            'version' => 'integer',
            'created_at' => 'datetime',
        ];
    }
}
