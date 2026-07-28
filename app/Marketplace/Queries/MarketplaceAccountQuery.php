<?php

namespace App\Marketplace\Queries;

use App\Marketplace\Models\AuctionWatch;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;
use Illuminate\Support\Collection;

final class MarketplaceAccountQuery
{
    /** @return array{wallet: WalletAccount, watched: Collection<int, CharacterAuction>, bids: Collection<int, CharacterAuction>, selling: Collection<int, CharacterAuction>, history: Collection<int, CharacterAuction>} */
    public function forIdentity(int $identityId): array
    {
        $wallet = WalletAccount::query()->firstOrCreate(
            ['identity_id' => $identityId],
            ['available_balance' => 0, 'reserved_balance' => 0],
        );

        $watchedIds = AuctionWatch::query()
            ->where('identity_id', $identityId)
            ->pluck('auction_id');

        $watched = CharacterAuction::query()
            ->whereIn('id', $watchedIds)
            ->orderByRaw('ends_at IS NULL')
            ->orderBy('ends_at')
            ->limit(100)
            ->get();

        $bids = CharacterAuction::query()
            ->whereHas('bids', fn ($query) => $query->where('bidder_identity_id', $identityId))
            ->orderByDesc('created_at')
            ->limit(100)
            ->get();

        $selling = CharacterAuction::query()
            ->where('seller_identity_id', $identityId)
            ->whereNotIn('status', [
                CharacterAuction::STATUS_COMPLETED,
                CharacterAuction::STATUS_CANCELLED,
                CharacterAuction::STATUS_EXPIRED,
            ])
            ->orderByDesc('created_at')
            ->limit(100)
            ->get();

        $history = CharacterAuction::query()
            ->where(function ($query) use ($identityId): void {
                $query->where('seller_identity_id', $identityId)
                    ->orWhere('highest_bidder_identity_id', $identityId);
            })
            ->whereIn('status', [
                CharacterAuction::STATUS_COMPLETED,
                CharacterAuction::STATUS_CANCELLED,
                CharacterAuction::STATUS_EXPIRED,
            ])
            ->orderByDesc('updated_at')
            ->limit(100)
            ->get();

        return compact('wallet', 'watched', 'bids', 'selling', 'history');
    }
}
