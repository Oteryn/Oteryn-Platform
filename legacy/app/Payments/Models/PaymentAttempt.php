<?php

namespace App\Payments\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $public_id
 * @property int $payment_order_id
 * @property string $provider
 * @property string $status
 * @property string|null $provider_checkout_reference
 * @property string $idempotency_key
 * @property string|null $sanitized_error_code
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class PaymentAttempt extends Model
{
    public const STATUS_CREATING = 'creating';

    public const STATUS_READY = 'ready';

    public const STATUS_AMBIGUOUS = 'ambiguous';

    /** @var list<string> */
    protected $fillable = [
        'public_id',
        'payment_order_id',
        'provider',
        'status',
        'provider_checkout_reference',
        'idempotency_key',
        'sanitized_error_code',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'payment_order_id' => 'integer',
        ];
    }
}
