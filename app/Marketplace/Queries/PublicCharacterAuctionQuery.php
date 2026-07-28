<?php

namespace App\Marketplace\Queries;

use App\Marketplace\Models\AuctionBid;
use App\Marketplace\Models\CharacterAuction;
use Illuminate\Contracts\Pagination\LengthAwarePaginator;
use Illuminate\Support\Collection;

final class PublicCharacterAuctionQuery
{
    /**
     * @param array{vocation?: int|null, level_min?: int|null, level_max?: int|null, price_min?: int|null, price_max?: int|null, sort?: string|null} $filters
     * @return LengthAwarePaginator<int, CharacterAuction>
     */
    public function paginate(array $filters, int $perPage = 24): LengthAwarePaginator
    {
        $query = CharacterAuction::query()->publicActive();

        if (isset($filters['vocation']) && $filters['vocation'] !== null) {
            $query->where('vocation', $filters['vocation']);
        }
        if (isset($filters['level_min']) && $filters['level_min'] !== null) {
            $query->where('level', '>=', $filters['level_min']);
        }
        if (isset($filters['level_max']) && $filters['level_max'] !== null) {
            $query->where('level', '<=', $filters['level_max']);
        }

        $effectivePrice = 'CASE WHEN current_bid > 0 THEN current_bid ELSE starting_bid END';
        if (isset($filters['price_min']) && $filters['price_min'] !== null) {
            $query->whereRaw("{$effectivePrice} >= ?", [$filters['price_min']]);
        }
        if (isset($filters['price_max']) && $filters['price_max'] !== null) {
            $query->whereRaw("{$effectivePrice} <= ?", [$filters['price_max']]);
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
        $limit = (int) config('marketplace.public_bid_history_limit', 20);
        $limit = max(1, min($limit, 100));

        return AuctionBid::query()
            ->where('auction_id', $auction->id)
            ->orderByDesc('placed_at')
            ->limit($limit)
            ->get(['id', 'auction_id', 'amount', 'status', 'placed_at']);
    }
}
