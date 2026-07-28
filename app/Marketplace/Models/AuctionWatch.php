<?php

namespace App\Marketplace\Models;

use Illuminate\Database\Eloquent\Model;

/**
 * @property int $identity_id
 * @property int $auction_id
 */
final class AuctionWatch extends Model
{
    public const UPDATED_AT = null;

    public $incrementing = false;

    protected $table = 'character_auction_watches';

    /** @var list<string> */
    protected $fillable = ['identity_id', 'auction_id', 'created_at'];

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'identity_id' => 'integer',
            'auction_id' => 'integer',
            'created_at' => 'datetime',
        ];
    }
}
