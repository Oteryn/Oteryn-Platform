<?php

namespace App\Marketplace\Queries;

use App\Marketplace\Exceptions\MarketplaceException;
use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Support\Collection;

final class PublicCharacterAuctionQuery
{
    /**
     * @param  array{vocation?: int|null, level_min?: int|null, level_max?: int|null, price_min?: int|null, price_max?: int|null, sort?: string|null}  $filters
     * @return LengthAwarePaginator<int, CharacterAuction>
     */
    public function paginate(array $filters, int $perPage = 24): LengthAwarePaginator
    {
        $query = CharacterAuction::query()->publicActive();
        $vocation = $filters['vocation'] ?? null;
        $levelMin = $filters['level_min'] ?? null;
        $levelMax = $filters['level_max'] ?? null;
        $priceMin = $filters['price_min'] ?? null;
        $priceMax = $filters['price_max'] ?? null;

        if ($vocation !== null) {
            $query->where('vocation', $vocation);
        }
        if ($levelMin !== null) {
            $query->where('level', '>=', $levelMin);
        }
        if ($levelMax !== null) {
            $query->where('level', '<=', $levelMax);
        }

        $effectivePrice = 'CASE WHEN current_bid > 0 THEN current_bid ELSE starting_bid END';
        if ($priceMin !== null) {
            $query->whereRaw("{$effectivePrice} >= ?", [$priceMin]);
        }
        if ($priceMax !== null) {
            $query->whereRaw("{$effectivePrice} <= ?", [$priceMax]);
        }

        match ($filters['sort'] ?? 'ending') {
            'newest' => $query->orderByDesc('starts_at')->orderBy('id'),
            'level_desc' => $query->orderByDesc('level')->orderBy('ends_at')->orderBy('id'),
            'price_asc' => $query->orderByRaw("{$effectivePrice} ASC")->orderBy('ends_at')->orderBy('id'),
            'price_desc' => $query->orderByRaw("{$effectivePrice} DESC")->orderBy('ends_at')->orderBy('id'),
            default => $query->orderBy('ends_at')->orderBy('id'),
        };

        return $query->paginate(max(1, min($perPage, 50)))->withQueryString();
    }

    public function visible(CharacterAuction $auction): bool
    {
        return in_array($auction->status, [
            CharacterAuction::STATUS_ACTIVE,
            CharacterAuction::STATUS_SETTLEMENT_PENDING,
            CharacterAuction::STATUS_COMPLETED,
            CharacterAuction::STATUS_CANCELLED,
            CharacterAuction::STATUS_EXPIRED,
        ], true);
    }

    /** @return Collection<int, AuctionBid> */
    public function publicBidHistory(CharacterAuction $auction): Collection
    {
        $configuredLimit = config('marketplace.public_bid_history_limit', 20);
        if (! is_int($configuredLimit) || $configuredLimit < 1 || $configuredLimit > 100) {
            throw new MarketplaceException('invalid_marketplace_configuration', 'The public bid history configuration is invalid.');
        }

        return AuctionBid::query()
            ->where('auction_id', $auction->id)
            ->orderByDesc('placed_at')
            ->limit($configuredLimit)
            ->get(['id', 'auction_id', 'amount', 'status', 'placed_at']);
    }

    /** @return list<int> */
    public function sitemapIds(): array
    {
        $auctions = CharacterAuction::query()
            ->whereIn('status', [
                CharacterAuction::STATUS_ACTIVE,
                CharacterAuction::STATUS_SETTLEMENT_PENDING,
                CharacterAuction::STATUS_COMPLETED,
            ])
            ->orderBy('id')
            ->limit(10_000)
            ->get(['id']);
        $ids = [];

        foreach ($auctions as $auction) {
            $ids[] = $auction->id;
        }

        return $ids;
    }
}
