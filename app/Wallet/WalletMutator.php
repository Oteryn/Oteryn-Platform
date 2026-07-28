<?php

namespace App\Wallet;

use App\Marketplace\Exceptions\MarketplaceException;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use Illuminate\Support\Facades\DB;

final class WalletMutator
{
    public function lock(int $identityId): WalletAccount
    {
        $now = now();
        DB::table('wallet_accounts')->insertOrIgnore([
            'identity_id' => $identityId,
            'available_balance' => 0,
            'reserved_balance' => 0,
            'created_at' => $now,
            'updated_at' => $now,
        ]);

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
     * @param  array<string, bool|int|string|null>  $metadata
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
        $normalizedMetadata = $metadata === [] ? null : $metadata;
        $existing = WalletLedgerEntry::query()
            ->where('idempotency_key', $idempotencyKey)
            ->first();

        if ($existing instanceof WalletLedgerEntry) {
            if ($existing->identity_id !== $wallet->identity_id
                || $existing->operation_type !== $operationType
                || $existing->available_delta !== $availableDelta
                || $existing->reserved_delta !== $reservedDelta
                || $existing->auction_id !== $auctionId
                || $existing->metadata !== $normalizedMetadata) {
                throw new MarketplaceException('idempotency_conflict', 'The wallet operation identifier is already in use.');
            }

            return false;
        }

        if ($availableDelta > 0 && $wallet->available_balance > PHP_INT_MAX - $availableDelta) {
            throw new MarketplaceException('wallet_balance_limit', 'The wallet balance limit would be exceeded.');
        }
        if ($reservedDelta > 0 && $wallet->reserved_balance > PHP_INT_MAX - $reservedDelta) {
            throw new MarketplaceException('wallet_balance_limit', 'The wallet balance limit would be exceeded.');
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
            'metadata' => $normalizedMetadata,
            'created_at' => now(),
        ]);

        return true;
    }
}
