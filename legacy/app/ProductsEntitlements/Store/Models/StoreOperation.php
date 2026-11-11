<?php

namespace App\ProductsEntitlements\Store\Models;

use Illuminate\Database\Eloquent\Model;

final class StoreOperation extends Model
{
    protected $table = 'store_operations';

    protected $primaryKey = 'source_revision';

    public $incrementing = true;

    protected $keyType = 'int';

    public $timestamps = false;

    protected $guarded = [];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'source_revision' => 'integer',
            'created_at' => 'immutable_datetime',
        ];
    }
}
