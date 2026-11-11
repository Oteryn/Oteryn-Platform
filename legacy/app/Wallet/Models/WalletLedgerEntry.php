<?php

namespace App\Wallet\Models;

use Illuminate\Database\Eloquent\Model;

/**
 * @property int $id
 * @property int $identity_id
 * @property string $operation_type
 * @property int $available_delta
 * @property int $reserved_delta
 * @property int|null $auction_id
 * @property string $idempotency_key
 * @property array<string, bool|int|string|null>|null $metadata
 */
final class WalletLedgerEntry extends Model
{
    public const UPDATED_AT = null;

    /** @var list<string> */
    protected $fillable = [
        'identity_id',
        'operation_type',
        'available_delta',
        'reserved_delta',
        'auction_id',
        'idempotency_key',
        'metadata',
        'created_at',
    ];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'available_delta' => 'integer',
            'reserved_delta' => 'integer',
            'auction_id' => 'integer',
            'metadata' => 'array',
            'created_at' => 'datetime',
        ];
    }
}
