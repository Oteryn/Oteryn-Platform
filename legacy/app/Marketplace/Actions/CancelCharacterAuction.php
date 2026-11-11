<?php

namespace App\Marketplace\Actions;

use App\Identity\Models\Identity;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Support\Facades\DB;

final class CancelCharacterAuction
{
    public function __construct(private readonly ReconcileCharacterAuctions $reconcile) {}

    public function execute(Identity $seller, CharacterAuction $auction): CharacterAuction
    {
        $pending = DB::transaction(function () use ($seller, $auction): CharacterAuction {
            $locked = CharacterAuction::query()->whereKey($auction->id)->lockForUpdate()->firstOrFail();

            if ($locked->seller_identity_id !== $seller->id) {
                throw new MarketplaceException('seller_required', 'Only the seller can cancel this auction.');
            }

            if (! in_array($locked->status, [CharacterAuction::STATUS_ESCROW_PENDING, CharacterAuction::STATUS_ACTIVE], true)) {
                throw new MarketplaceException('auction_not_cancellable', 'This auction can no longer be cancelled.');
            }

            if ($locked->highest_bidder_identity_id !== null || $locked->bid_count !== 0) {
                throw new MarketplaceException('auction_has_bids', 'An auction with bids cannot be cancelled.');
            }

            $locked->status = CharacterAuction::STATUS_CANCEL_PENDING;
            $locked->saga_state = CharacterAuction::SAGA_RETURN_TO_SELLER;
            $locked->failure_code = null;
            $locked->lock_version++;
            $locked->save();

            return $locked;
        }, 3);

        return $this->reconcile->reconcile($pending);
    }
}
