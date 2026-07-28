<?php

namespace App\Marketplace\Models;

use App\Identity\Models\Identity;
use App\Marketplace\Exceptions\MarketplaceException;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;
use Illuminate\Database\Eloquent\Relations\HasMany;
use Illuminate\Support\Carbon;

/**
 * @property int $id
 * @property string $listing_request_id
 * @property int $seller_identity_id
 * @property int $seller_canary_account_id
 * @property int $escrow_canary_account_id
 * @property int $player_id
 * @property int|null $active_player_id
 * @property string $player_name
 * @property int $level
 * @property int $vocation
 * @property int $sex
 * @property array<string, int|string|null> $character_snapshot
 * @property string $status
 * @property string $saga_state
 * @property string|null $failure_code
 * @property int $duration_days
 * @property int $starting_bid
 * @property int|null $buy_now_price
 * @property int $current_bid
 * @property int|null $highest_bidder_identity_id
 * @property int $bid_count
 * @property int $lock_version
 * @property Carbon|null $starts_at
 * @property Carbon|null $ends_at
 * @property Carbon|null $escrowed_at
 * @property Carbon|null $settlement_started_at
 * @property Carbon|null $settled_at
 * @property Carbon|null $cancelled_at
 */
final class CharacterAuction extends Model
{
    public const STATUS_ESCROW_PENDING = 'escrow_pending';

    public const STATUS_ACTIVE = 'active';

    public const STATUS_SETTLEMENT_PENDING = 'settlement_pending';

    public const STATUS_COMPLETED = 'completed';

    public const STATUS_CANCEL_PENDING = 'cancel_pending';

    public const STATUS_CANCELLED = 'cancelled';

    public const STATUS_EXPIRED = 'expired';

    public const STATUS_RECOVERY_REQUIRED = 'recovery_required';

    public const SAGA_ESCROW_REQUESTED = 'escrow_requested';

    public const SAGA_QUIESCENCE_WAIT = 'quiescence_wait';

    public const SAGA_ACTIVE = 'active';

    public const SAGA_TRANSFER_TO_WINNER = 'transfer_to_winner';

    public const SAGA_WALLET_SETTLEMENT = 'wallet_settlement';

    public const SAGA_RETURN_TO_SELLER = 'return_to_seller';

    public const SAGA_DONE = 'done';

    public const SAGA_RECOVERY_REQUIRED = 'recovery_required';

    /** @var list<string> */
    protected $fillable = [
        'listing_request_id',
        'seller_identity_id',
        'seller_canary_account_id',
        'escrow_canary_account_id',
        'player_id',
        'active_player_id',
        'player_name',
        'level',
        'vocation',
        'sex',
        'character_snapshot',
        'status',
        'saga_state',
        'failure_code',
        'duration_days',
        'starting_bid',
        'buy_now_price',
        'current_bid',
        'highest_bidder_identity_id',
        'bid_count',
        'lock_version',
        'starts_at',
        'ends_at',
        'escrowed_at',
        'settlement_started_at',
        'settled_at',
        'cancelled_at',
    ];

    /** @return BelongsTo<Identity, $this> */
    public function seller(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'seller_identity_id');
    }

    /** @return BelongsTo<Identity, $this> */
    public function highestBidder(): BelongsTo
    {
        return $this->belongsTo(Identity::class, 'highest_bidder_identity_id');
    }

    /** @return HasMany<AuctionBid, $this> */
    public function bids(): HasMany
    {
        return $this->hasMany(AuctionBid::class, 'auction_id');
    }

    /** @param Builder<CharacterAuction> $query */
    public function scopePublicActive(Builder $query): void
    {
        $query->where('status', self::STATUS_ACTIVE)
            ->whereNotNull('starts_at')
            ->whereNotNull('ends_at')
            ->where('ends_at', '>', now());
    }

    public function minimumNextBid(): int
    {
        if ($this->current_bid === 0) {
            return $this->starting_bid;
        }

        $configuredIncrement = config('marketplace.minimum_bid_increment', 10);
        if (! is_int($configuredIncrement) || $configuredIncrement < 1) {
            throw new MarketplaceException('invalid_marketplace_configuration', 'The marketplace bid increment configuration is invalid.');
        }

        return $this->current_bid + $configuredIncrement;
    }

    public function isTerminal(): bool
    {
        return in_array($this->status, [self::STATUS_COMPLETED, self::STATUS_CANCELLED, self::STATUS_EXPIRED], true);
    }

    /** @return array<string, string> */
    protected function casts(): array
    {
        return [
            'seller_identity_id' => 'integer',
            'seller_canary_account_id' => 'integer',
            'escrow_canary_account_id' => 'integer',
            'player_id' => 'integer',
            'active_player_id' => 'integer',
            'level' => 'integer',
            'vocation' => 'integer',
            'sex' => 'integer',
            'character_snapshot' => 'array',
            'duration_days' => 'integer',
            'starting_bid' => 'integer',
            'buy_now_price' => 'integer',
            'current_bid' => 'integer',
            'highest_bidder_identity_id' => 'integer',
            'bid_count' => 'integer',
            'lock_version' => 'integer',
            'starts_at' => 'datetime',
            'ends_at' => 'datetime',
            'escrowed_at' => 'datetime',
            'settlement_started_at' => 'datetime',
            'settled_at' => 'datetime',
            'cancelled_at' => 'datetime',
        ];
    }
}
