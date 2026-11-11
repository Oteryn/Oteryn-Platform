<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

final class StoreOrderLine extends Model
{
    protected $table = 'store_order_lines';

    protected $primaryKey = 'line_id';

    public $incrementing = false;

    protected $keyType = 'string';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'quantity' => 'integer',
            'unit_price' => 'integer',
        ];
    }
}
