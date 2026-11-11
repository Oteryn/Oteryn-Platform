<?php

namespace App\Marketplace\Http;

use App\Identity\Models\Identity;
use App\Marketplace\Queries\MarketplaceAccountQuery;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final class MarketplaceAccountController
{
    public function __construct(private readonly MarketplaceAccountQuery $marketplace) {}

    public function __invoke(Request $request): View
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return view('marketplace.account', $this->marketplace->forIdentity($identity->id));
    }
}
