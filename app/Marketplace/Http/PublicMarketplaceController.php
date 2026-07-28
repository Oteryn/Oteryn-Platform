<?php

namespace App\Marketplace\Http;

use App\Marketplace\Models\CharacterAuction;
use App\Marketplace\Queries\PublicCharacterAuctionQuery;
use Illuminate\Contracts\View\View;
use Illuminate\Http\Request;

final class PublicMarketplaceController
{
    public function __construct(private readonly PublicCharacterAuctionQuery $auctions) {}

    public function index(Request $request): View
    {
        /** @var array{vocation?: int|null, level_min?: int|null, level_max?: int|null, price_min?: int|null, price_max?: int|null, sort?: string|null} $filters */
        $filters = $request->validate([
            'vocation' => ['nullable', 'integer', 'min:0', 'max:20'],
            'level_min' => ['nullable', 'integer', 'min:1', 'max:5000'],
            'level_max' => ['nullable', 'integer', 'min:1', 'max:5000', 'gte:level_min'],
            'price_min' => ['nullable', 'integer', 'min:0', 'max:1000000000'],
            'price_max' => ['nullable', 'integer', 'min:0', 'max:1000000000', 'gte:price_min'],
            'sort' => ['nullable', 'in:ending,newest,level_desc,price_asc,price_desc'],
        ]);

        return view('marketplace.index', [
            'auctions' => $this->auctions->paginate($filters),
            'filters' => $filters,
        ]);
    }

    public function show(CharacterAuction $auction): View
    {
        abort_unless($this->auctions->visible($auction), 404);

        return view('marketplace.show', [
            'auction' => $auction,
            'bids' => $this->auctions->publicBidHistory($auction),
        ]);
    }
}
