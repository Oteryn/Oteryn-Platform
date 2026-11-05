<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

final class StoreReceipt extends Model
{
    protected $table = 'store_receipts';

    protected $primaryKey = 'id';

    public $incrementing = true;

    protected $keyType = 'int';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'recovery_generation' => 'string',
            'decided_at' => 'string',
            'received_at' => 'immutable_datetime',
        ];
    }
}
