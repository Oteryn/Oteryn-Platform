<?php

namespace App\Marketplace\Http;

use App\Identity\Models\Identity;
use App\Marketplace\Models\AuctionWatch;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;

final class MarketplaceWatchController
{
    public function store(Request $request, CharacterAuction $auction): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        AuctionWatch::query()->firstOrCreate([
            'identity_id' => $identity->id,
            'auction_id' => $auction->id,
        ], ['created_at' => now()]);

        return back()->with('status', __('marketplace.watch_added'));
    }

    public function destroy(Request $request, CharacterAuction $auction): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        AuctionWatch::query()
            ->where('identity_id', $identity->id)
            ->where('auction_id', $auction->id)
            ->delete();

        return back()->with('status', __('marketplace.watch_removed'));
    }
}
