<?php

namespace App\Marketplace\Http;

use App\Identity\Models\Identity;
use App\Marketplace\Actions\PlaceAuctionBid;
use App\Marketplace\Actions\ReconcileCharacterAuctions;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

final class MarketplaceBidController
{
    public function __construct(
        private readonly PlaceAuctionBid $bids,
        private readonly ReconcileCharacterAuctions $reconcile,
    ) {}

    public function store(Request $request, CharacterAuction $auction): RedirectResponse
    {
        return $this->place($request, $auction, false);
    }

    public function purchase(Request $request, CharacterAuction $auction): RedirectResponse
    {
        return $this->place($request, $auction, true);
    }

    private function place(Request $request, CharacterAuction $auction, bool $fixedPrice): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        /** @var array{amount: int, request_id: string} $validated */
        $validated = $request->validate([
            'amount' => ['required', 'integer', 'min:1', 'max:1000000000'],
            'request_id' => ['required', 'uuid'],
        ]);

        try {
            $updated = $this->bids->execute(
                $identity,
                $auction,
                $validated['amount'],
                $validated['request_id'],
                $fixedPrice,
            );

            if ($fixedPrice) {
                $this->reconcile->reconcile($updated);
            }
        } catch (MarketplaceException $exception) {
            return back()->withInput()->withErrors(['marketplace' => $exception->getMessage()]);
        }

        return back()->with('status', $fixedPrice ? __('marketplace.buy_now_processing') : __('marketplace.bid_placed'));
    }
}
