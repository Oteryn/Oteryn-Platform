<?php

namespace App\Identity\Termination;

use App\Identity\Models\Identity;
use App\Identity\Models\IdentityEmailChangeRequest;
use App\Marketplace\Models\CharacterAuction;
use App\Wallet\Models\WalletAccount;

final class IdentityTerminationGuard
{
    public function assertAvailable(Identity $identity): void
    {
        $pendingEmailChange = IdentityEmailChangeRequest::query()
            ->where('identity_id', $identity->id)
            ->whereNull('confirmed_at')
            ->whereNull('cancelled_at')
            ->whereNull('recovered_at')
            ->where('expires_at', '>', now())
            ->exists();
        if ($pendingEmailChange) {
            throw new AccountTerminationRejected(__('identity.errors.termination_pending_email'));
        }

        $nonTerminalStatuses = [
            CharacterAuction::STATUS_ESCROW_PENDING,
            CharacterAuction::STATUS_ACTIVE,
            CharacterAuction::STATUS_SETTLEMENT_PENDING,
            CharacterAuction::STATUS_CANCEL_PENDING,
            CharacterAuction::STATUS_RECOVERY_REQUIRED,
        ];
        $activeMarketplaceOperation = CharacterAuction::query()
            ->whereIn('status', $nonTerminalStatuses)
            ->where(function ($query) use ($identity): void {
                $query->where('seller_identity_id', $identity->id)
                    ->orWhere('highest_bidder_identity_id', $identity->id);
            })
            ->exists();
        if ($activeMarketplaceOperation) {
            throw new AccountTerminationRejected(__('identity.errors.termination_active_bazaar'));
        }

        $reservedBalance = WalletAccount::query()
            ->where('identity_id', $identity->id)
            ->value('reserved_balance');
        if (is_int($reservedBalance) && $reservedBalance > 0) {
            throw new AccountTerminationRejected(__('identity.errors.termination_reserved_coins'));
        }
    }
}
