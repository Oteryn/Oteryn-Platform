<?php

namespace App\Wallet;

use App\Marketplace\Exceptions\MarketplaceException;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;

final class WalletMutator
{
    public function lock(int $identityId): WalletAccount
    {
        WalletAccount::query()->firstOrCreate(
            ['identity_id' => $identityId],
            ['available_balance' => 0, 'reserved_balance' => 0],
        );

        $wallet = WalletAccount::query()
            ->whereKey($identityId)
            ->lockForUpdate()
            ->first();

        if (! $wallet instanceof WalletAccount) {
            throw new MarketplaceException('wallet_unavailable', 'The wallet is temporarily unavailable.');
        }

        return $wallet;
    }

    /**
     * @param array<string, bool|int|string|null> $metadata
     */
    public function applyLocked(
        WalletAccount $wallet,
        string $operationType,
        int $availableDelta,
        int $reservedDelta,
        string $idempotencyKey,
        ?int $auctionId = null,
        array $metadata = [],
    ): bool {
        if (WalletLedgerEntry::query()->where('idempotency_key', $idempotencyKey)->exists()) {
            return false;
        }

        $available = $wallet->available_balance + $availableDelta;
        $reserved = $wallet->reserved_balance + $reservedDelta;

        if ($available < 0) {
            throw new MarketplaceException('insufficient_balance', 'Your available Oteryn Coins balance is too low.');
        }

        if ($reserved < 0) {
            throw new MarketplaceException('wallet_reservation_conflict', 'The wallet reservation state is inconsistent.');
        }

        $wallet->available_balance = $available;
        $wallet->reserved_balance = $reserved;
        $wallet->save();

        WalletLedgerEntry::query()->create([
            'identity_id' => $wallet->identity_id,
            'operation_type' => $operationType,
            'available_delta' => $availableDelta,
            'reserved_delta' => $reservedDelta,
            'auction_id' => $auctionId,
            'idempotency_key' => $idempotencyKey,
            'metadata' => $metadata === [] ? null : $metadata,
            'created_at' => now(),
        ]);

        return true;
    }
}
