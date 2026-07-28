<?php

namespace App\Marketplace\Models;

use App\Identity\Models\Identity;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $request_id
 * @property int $auction_id
 * @property int $bidder_identity_id
 * @property int $amount
 * @property string $status
 * @property Carbon $placed_at
 */
final class AuctionBid extends Model
{
    public const CREATED_AT = 'placed_at';

    public const STATUS_LEADING = 'leading';

    public const STATUS_OUTBID = 'outbid';

    public const STATUS_WON = 'won';

    public const STATUS_RELEASED = 'released';

    /** @var list<string> */
    protected $fillable = [
        'request_id',
        'auction_id',
        'bidder_identity_id',
        'amount',
        'status',
        'placed_at',
        'updated_at',
    ];

    /** @return BelongsTo<CharacterAuction, $this> */
    public function auction(): BelongsTo
    {
        return $this->belongsTo(CharacterAuction::class, 'auction_id');
    }

    /** @return BelongsTo<Identity, $this> */
    public function bidder(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'bidder_identity_id');
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'auction_id' => 'integer',
            'bidder_identity_id' => 'integer',
            'amount' => 'integer',
            'placed_at' => 'datetime',
            'updated_at' => 'datetime',
        ];
    }
}
