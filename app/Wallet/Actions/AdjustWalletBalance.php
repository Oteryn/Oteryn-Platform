<?php

namespace App\Wallet\Actions;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Wallet\Models\WalletAccount;
use App\Wallet\Models\WalletLedgerEntry;
use App\Wallet\WalletMutator;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;

final class AdjustWalletBalance
{
    public function __construct(
        private readonly WalletMutator $wallets,
        private readonly AdminAuditRecorder $audit,
    ) {}

    public function execute(
        Identity $actor,
        Identity $target,
        int $amount,
        string $reason,
        string $requestId,
    ): WalletAccount {
        $reason = trim($reason);
        if ($amount === 0 || abs($amount) > 1_000_000_000) {
            throw new MarketplaceException('wallet_adjustment_invalid', 'The balance adjustment must be non-zero and within the approved limit.');
        }
        if (mb_strlen($reason) < 10 || mb_strlen($reason) > 500) {
            throw new MarketplaceException('wallet_adjustment_reason_invalid', 'Provide a reason between 10 and 500 characters.');
        }

        $reasonHash = hash('sha256', $reason);
        $idempotencyKey = "admin-wallet-adjustment:{$requestId}";

        try {
            return DB::transaction(function () use ($actor, $target, $amount, $reasonHash, $requestId, $idempotencyKey): WalletAccount {
                $locked = $this->wallets->lock($target->id);
                $existing = WalletLedgerEntry::query()->where('idempotency_key', $idempotencyKey)->first();
                if ($existing instanceof WalletLedgerEntry) {
                    return $this->existingResult($existing, $locked, $actor, $target, $amount, $reasonHash);
                }

                $applied = $this->wallets->applyLocked(
                    $locked,
                    'administrator_adjustment',
                    $amount,
                    0,
                    $idempotencyKey,
                    null,
                    [
                        'actor_identity_id' => $actor->id,
                        'reason_sha256' => $reasonHash,
                    ],
                );

                if (! $applied) {
                    $existing = WalletLedgerEntry::query()->where('idempotency_key', $idempotencyKey)->first();
                    if (! $existing instanceof WalletLedgerEntry) {
                        throw new MarketplaceException('wallet_unavailable', 'The wallet is temporarily unavailable.');
                    }

                    return $this->existingResult($existing, $locked, $actor, $target, $amount, $reasonHash);
                }

                $this->audit->record(
                    $actor->id,
                    'marketplace.wallet_adjusted',
                    'identity_wallet',
                    (string) $target->id,
                    [
                        'amount' => $amount,
                        'reason_sha256' => $reasonHash,
                        'request_id' => $requestId,
                    ],
                );

                return $locked->refresh();
            }, 3);
        } catch (QueryException $exception) {
            if ($this->isDuplicateKey($exception)) {
                $existing = WalletLedgerEntry::query()->where('idempotency_key', $idempotencyKey)->first();
                if ($existing instanceof WalletLedgerEntry) {
                    $wallet = WalletAccount::query()->whereKey($target->id)->first();
                    if (! $wallet instanceof WalletAccount) {
                        throw new MarketplaceException('wallet_unavailable', 'The wallet is temporarily unavailable.');
                    }

                    return $this->existingResult($existing, $wallet, $actor, $target, $amount, $reasonHash);
                }
            }

            throw new MarketplaceException('dependency_unavailable', 'The marketplace database is temporarily unavailable.');
        }
    }

    private function existingResult(
        WalletLedgerEntry $existing,
        WalletAccount $wallet,
        Identity $actor,
        Identity $target,
        int $amount,
        string $reasonHash,
    ): WalletAccount {
        $metadata = $existing->metadata ?? [];
        if ($existing->identity_id !== $target->id
            || $existing->operation_type !== 'administrator_adjustment'
            || $existing->available_delta !== $amount
            || $existing->reserved_delta !== 0
            || $existing->auction_id !== null
            || ($metadata['actor_identity_id'] ?? null) !== $actor->id
            || ($metadata['reason_sha256'] ?? null) !== $reasonHash) {
            throw new MarketplaceException('idempotency_conflict', 'The wallet adjustment request identifier is already in use.');
        }

        return $wallet->refresh();
    }

    private function isDuplicateKey(QueryException $exception): bool
    {
        $driverCode = $exception->errorInfo[1] ?? null;

        return (string) $exception->getCode() === '23000'
            && (is_int($driverCode) || is_string($driverCode))
            && (int) $driverCode === 1062;
    }
}
