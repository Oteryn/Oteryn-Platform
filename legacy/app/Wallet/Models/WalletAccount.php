<?php

namespace App\Wallet\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

/**
 * @property int $identity_id
 * @property int $available_balance
 * @property int $reserved_balance
 */
final class WalletAccount extends Model
{
    protected $primaryKey = 'identity_id';

    public $incrementing = false;

    /** @var list<string> */
    protected $fillable = [
        'identity_id',
        'available_balance',
        'reserved_balance',
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
            'available_balance' => 'integer',
            'reserved_balance' => 'integer',
        ];
    }
}
