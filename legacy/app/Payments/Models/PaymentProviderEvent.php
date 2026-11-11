<?php

namespace App\Payments\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property int|null $payment_order_id
 * @property string $provider
 * @property string $provider_event_id
 * @property string $event_type
 * @property string|null $provider_object_reference
 * @property string $payload_sha256
 * @property int $signature_timestamp
 * @property string $processing_state
 * @property string|null $failure_code
 * @property array<string, bool|int|string|null>|null $metadata
 * @property Carbon $received_at
 * @property Carbon|null $processed_at
 */
final class PaymentProviderEvent extends Model
{
    public const CREATED_AT = 'received_at';

    public const UPDATED_AT = null;

    public const STATE_PROCESSED = 'processed';

    public const STATE_RECONCILIATION = 'reconciliation';

    /** @var list<string> */
    protected $fillable = [
        'payment_order_id',
        'provider',
        'provider_event_id',
        'event_type',
        'provider_object_reference',
        'payload_sha256',
        'signature_timestamp',
        'processing_state',
        'failure_code',
        'metadata',
        'received_at',
        'processed_at',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'payment_order_id' => 'integer',
            'signature_timestamp' => 'integer',
            'metadata' => 'array',
            'received_at' => 'datetime',
            'processed_at' => 'datetime',
        ];
    }
}
