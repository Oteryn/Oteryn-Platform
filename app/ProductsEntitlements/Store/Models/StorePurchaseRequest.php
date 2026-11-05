<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

final class StorePurchaseRequest extends Model
{
    protected $table = 'store_purchase_requests';

    protected $primaryKey = 'purchase_request_id';

    public $incrementing = false;

    protected $keyType = 'string';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'unit_price' => 'integer',
            'quantity' => 'integer',
            'decided_at' => 'immutable_datetime',
        ];
    }
}
