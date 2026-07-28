<?php

namespace App\Marketplace\Http;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use App\Marketplace\Actions\CancelCharacterAuction;
use App\Marketplace\Actions\CreateCharacterAuction;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\CharacterAuction;
use App\PublicGameData\CanaryGameDataRepository;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Collection;
use Throwable;

final class MarketplaceListingController
{
    public function __construct(
        private readonly CanaryGameDataRepository $canary,
        private readonly CreateCharacterAuction $create,
        private readonly CancelCharacterAuction $cancel,
    ) {}

    public function create(Request $request): View
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->first();
        $bindingReady = $binding !== null && $binding->isReady() && is_int($binding->canary_account_id);
        /** @var Collection<int, object> $characters */
        $characters = collect();
        $charactersAvailable = true;

        if ($bindingReady) {
            try {
                $characters = $this->canary->activeCharactersForAccount($binding->canary_account_id);
            } catch (Throwable) {
                $charactersAvailable = false;
            }
        }

        return view('marketplace.create', [
            'bindingReady' => $bindingReady,
            'charactersAvailable' => $charactersAvailable,
            'characters' => $characters,
            'durations' => config('marketplace.allowed_duration_days', [1, 3, 7]),
        ]);
    }

    public function store(Request $request): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        $validated = $request->validate([
            'player_id' => ['required', 'integer', 'min:1'],
            'duration_days' => ['required', 'integer'],
            'starting_bid' => ['required', 'integer', 'min:1', 'max:1000000000'],
            'buy_now_price' => ['nullable', 'integer', 'min:1', 'max:1000000000'],
            'request_id' => ['required', 'uuid'],
        ]);

        try {
            $auction = $this->create->execute(
                $identity,
                (int) $validated['player_id'],
                (int) $validated['duration_days'],
                (int) $validated['starting_bid'],
                isset($validated['buy_now_price']) ? (int) $validated['buy_now_price'] : null,
                (string) $validated['request_id'],
            );
        } catch (MarketplaceException $exception) {
            return back()->withInput()->withErrors(['marketplace' => $exception->getMessage()]);
        }

        return redirect()->route('marketplace.account')
            ->with('status', __('marketplace.listing_pending', ['name' => $auction->player_name]));
    }

    public function cancel(Request $request, CharacterAuction $auction): RedirectResponse
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        try {
            $this->cancel->execute($identity, $auction);
        } catch (MarketplaceException $exception) {
            return back()->withErrors(['marketplace' => $exception->getMessage()]);
        }

        return redirect()->route('marketplace.account')->with('status', __('marketplace.cancel_requested'));
    }
}
