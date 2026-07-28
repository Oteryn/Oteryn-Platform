<?php

namespace App\Marketplace\Actions;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\WalletMutator;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;

final class PlaceAuctionBid
{
    public function __construct(private readonly WalletMutator $wallets) {}

    public function execute(
        Identity $bidder,
        CharacterAuction $auction,
        int $amount,
        string $requestId,
        bool $buyNow = false,
    ): CharacterAuction {
        $existing = AuctionBid::query()->where('request_id', $requestId)->first();
        if ($existing instanceof AuctionBid) {
            return $this->existingResult($existing, $bidder, $auction, $amount, $buyNow);
        }

        $binding = IdentityCanaryAccount::query()->whereKey($bidder->id)->first();
        if ($binding === null || ! $binding->isReady() || ! is_int($binding->canary_account_id)) {
            throw new MarketplaceException('binding_not_ready', 'Your game account must be ready before bidding.');
        }

        try {
            return DB::transaction(function () use ($bidder, $auction, $amount, $requestId, $buyNow): CharacterAuction {
                $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->first();
                if (! $locked instanceof CharacterAuction) {
                    throw new MarketplaceException('auction_missing', 'The auction no longer exists.');
                }

                $existing = AuctionBid::query()->where('request_id', $requestId)->first();
                if ($existing instanceof AuctionBid) {
                    return $this->existingResult($existing, $bidder, $locked, $amount, $buyNow);
                }

                if ($locked->status !== CharacterAuction::STATUS_ACTIVE || $locked->ends_at === null || ! $locked->ends_at->isFuture()) {
                    throw new MarketplaceException('auction_not_active', 'This auction is not accepting bids.');
                }

                if ($locked->seller_identity_id === $bidder->id) {
                    throw new MarketplaceException('self_bid_forbidden', 'You cannot bid on your own character.');
                }

                if ($buyNow) {
                    if ($locked->buy_now_price === null) {
                        throw new MarketplaceException('buy_now_unavailable', 'This auction does not have a buy-now price.');
                    }
                    if ($amount !== $locked->buy_now_price) {
                        throw new MarketplaceException('buy_now_amount_invalid', 'The buy-now amount changed. Reload the auction.');
                    }
                } elseif ($amount < $locked->minimumNextBid()) {
                    throw new MarketplaceException('bid_too_low', 'The bid does not meet the current minimum.');
                }

                if ($locked->buy_now_price !== null && $amount > $locked->buy_now_price) {
                    throw new MarketplaceException('bid_above_buy_now', 'The bid cannot exceed the buy-now price.');
                }

                $previousBid = AuctionBid::query()
                    ->where('auction_id', $locked->id)
                    ->where('status', AuctionBid::STATUS_LEADING)
                    ->lockForUpdate()
                    ->first();

                if ($locked->highest_bidder_identity_id !== null && ! $previousBid instanceof AuctionBid) {
                    throw new MarketplaceException('auction_state_conflict', 'The auction bid state requires recovery.');
                }

                $walletIdentityIds = [$bidder->id];
                if ($previousBid instanceof AuctionBid) {
                    $walletIdentityIds[] = $previousBid->bidder_identity_id;
                }
                $walletIdentityIds = array_values(array_unique($walletIdentityIds));
                sort($walletIdentityIds, SORT_NUMERIC);

                $wallets = [];
                foreach ($walletIdentityIds as $identityId) {
                    $wallets[$identityId] = $this->wallets->lock($identityId);
                }

                $newBid = AuctionBid::query()->create([
                    'request_id' => $requestId,
                    'auction_id' => $locked->id,
                    'bidder_identity_id' => $bidder->id,
                    'amount' => $amount,
                    'is_buy_now' => $buyNow,
                    'status' => AuctionBid::STATUS_LEADING,
                    'placed_at' => now(),
                    'updated_at' => now(),
                ]);

                if ($previousBid instanceof AuctionBid) {
                    $previousBid->status = AuctionBid::STATUS_OUTBID;
                    $previousBid->updated_at = now();
                    $previousBid->save();

                    $this->wallets->applyLocked(
                        $wallets[$previousBid->bidder_identity_id],
                        'auction_bid_released',
                        $previousBid->amount,
                        -$previousBid->amount,
                        "auction:{$locked->id}:bid:{$previousBid->id}:release:{$requestId}",
                        $locked->id,
                        ['replaced_by_bid_id' => $newBid->id],
                    );
                }

                $this->wallets->applyLocked(
                    $wallets[$bidder->id],
                    'auction_bid_reserved',
                    -$amount,
                    $amount,
                    "auction:{$locked->id}:bid:{$newBid->id}:reserve",
                    $locked->id,
                    ['bid_id' => $newBid->id],
                );

                $locked->current_bid = $amount;
                $locked->highest_bidder_identity_id = $bidder->id;
                $locked->bid_count++;
                $locked->lock_version++;

                if ($buyNow) {
                    $locked->status = CharacterAuction::STATUS_SETTLEMENT_PENDING;
                    $locked->saga_state = CharacterAuction::SAGA_TRANSFER_TO_WINNER;
                    $locked->settlement_started_at = now();
                }

                $locked->save();

                return $locked;
            }, 3);
        } catch (QueryException $exception) {
            if ($this->isDuplicateKey($exception)) {
                $existing = AuctionBid::query()->where('request_id', $requestId)->first();
                if ($existing instanceof AuctionBid) {
                    return $this->existingResult($existing, $bidder, $auction, $amount, $buyNow);
                }
            }

            throw new MarketplaceException('dependency_unavailable', 'The marketplace database is temporarily unavailable.');
        }
    }

    private function existingResult(
        AuctionBid $existing,
        Identity $bidder,
        CharacterAuction $auction,
        int $amount,
        bool $buyNow,
    ): CharacterAuction {
        if ($existing->bidder_identity_id !== $bidder->id
            || $existing->auction_id !== $auction->id
            || $existing->amount !== $amount
            || $existing->is_buy_now !== $buyNow) {
            throw new MarketplaceException('idempotency_conflict', 'The bid request identifier is already in use.');
        }

        return CharacterAuction::query()->findOrFail($auction->id);
    }

    private function isDuplicateKey(QueryException $exception): bool
    {
        $driverCode = $exception->errorInfo[1] ?? null;

        return (string) $exception->getCode() === '23000'
            && (is_int($driverCode) || is_string($driverCode))
            && (int) $driverCode === 1062;
    }
}
