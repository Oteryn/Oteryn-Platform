<?php

namespace App\Payments\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int $payment_reconciliation_entry_id
 * @property int $actor_identity_id
 * @property string $resolution_code
 * @property string $idempotency_key
 * @property Carbon $created_at
 */
final class PaymentReconciliationResolution extends Model
{
    public const UPDATED_AT = null;

    public const REVIEWED_NO_PAYMENT_STATE_CHANGE = 'reviewed_no_payment_state_change';

    /** @var list<string> */
    protected $fillable = [
        'payment_reconciliation_entry_id',
        'actor_identity_id',
        'resolution_code',
        'idempotency_key',
        'created_at',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'payment_reconciliation_entry_id' => 'integer',
            'actor_identity_id' => 'integer',
            'created_at' => 'datetime',
        ];
    }
}
