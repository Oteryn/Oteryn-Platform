<?php

namespace App\Payments\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $public_id
 * @property int $identity_id
 * @property string $provider
 * @property string $currency
 * @property int $amount_minor
 * @property string $status
 * @property string $idempotency_key
 * @property int $version
 * @property Carbon $created_at
 * @property Carbon $updated_at
 */
final class PaymentOrder extends Model
{
    public const STATUS_PENDING = 'pending';

    public const STATUS_CHECKOUT_CREATED = 'checkout_created';

    public const STATUS_SUCCEEDED = 'succeeded';

    public const STATUS_FAILED = 'failed';

    public const STATUS_CANCELLED = 'cancelled';

    public const STATUS_EXPIRED = 'expired';

    public const STATUS_PARTIALLY_REFUNDED = 'partially_refunded';

    public const STATUS_REFUNDED = 'refunded';

    public const STATUS_DISPUTED = 'disputed';

    public const STATUS_CHARGED_BACK = 'charged_back';

    /** @var list<string> */
    protected $fillable = [
        'public_id',
        'identity_id',
        'provider',
        'currency',
        'amount_minor',
        'status',
        'idempotency_key',
        'version',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'amount_minor' => 'integer',
            'version' => 'integer',
        ];
    }
}
