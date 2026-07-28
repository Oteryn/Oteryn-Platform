<?php

namespace App\Wallet\Actions;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Marketplace\Exceptions\MarketplaceException;
use App\Wallet\Models\WalletAccount;
use App\Wallet\WalletMutator;
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

        return DB::transaction(function () use ($actor, $target, $amount, $reason, $requestId): WalletAccount {
            $locked = $this->wallets->lock($target->id);
            $this->wallets->applyLocked(
                $locked,
                'administrator_adjustment',
                $amount,
                0,
                "admin-wallet-adjustment:{$requestId}",
                null,
                ['reason_sha256' => hash('sha256', $reason)],
            );

            $this->audit->record(
                $actor->id,
                'marketplace.wallet_adjusted',
                'identity_wallet',
                (string) $target->id,
                [
                    'amount' => $amount,
                    'reason_sha256' => hash('sha256', $reason),
                    'request_id' => $requestId,
                ],
            );

            return $locked->refresh();
        }, 3);
    }
}
