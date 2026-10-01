<?php

namespace App\ProductsEntitlements\Premium;

use Illuminate\Support\Facades\DB;
use JsonException;

/** Issues `oteryn.premium_snapshot.v1` for one account (contract sections 5 and 6). */
final class PremiumSnapshotIssuer
{
    public function __construct(private readonly PremiumTimeAuthority $authority) {}

    /**
     * Null when no Platform Identity has the AccountId; decided before any authority revision is allocated.
     *
     * @return array<string, int|string|null>|null
     */
    public function issue(string $accountId, string $nonce, string $producerRevision): ?array
    {
        $key = DB::table('identities')->where('account_id', $accountId)->value('id');
        if ($key === null) {
            return null;
        }
        $identityId = PremiumTimeContract::databaseInteger($key);
        if ($identityId === null) {
            throw new PremiumSnapshotUnavailable('Identity key is invalid.');
        }

        $snapshot = DB::transaction(function () use ($identityId, $accountId, $nonce, $producerRevision): array {
            $current = $this->authority->lock($identityId);
            $authorityRevision = $this->authority->advance($identityId, $current);
            $entitlement = PremiumTimeEntitlement::forIdentity($identityId);
            $issued = now()->getTimestamp();

            return $this->snapshot($entitlement, $issued, $authorityRevision, $accountId, $nonce, $producerRevision);
        }, 3);

        try {
            $bytes = strlen(json_encode($snapshot, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));
        } catch (JsonException) {
            throw new PremiumSnapshotUnavailable('Premium snapshot could not be encoded.');
        }
        if ($bytes > PremiumTimeContract::MAX_RESPONSE_BYTES) {
            throw new PremiumSnapshotUnavailable('Premium snapshot exceeds its size bound.');
        }

        return $snapshot;
    }

    /** @return array<string, int|string|null> */
    private function snapshot(
        ?PremiumTimeEntitlement $entitlement,
        int $issued,
        int $authorityRevision,
        string $accountId,
        string $nonce,
        string $producerRevision,
    ): array {
        $state = $entitlement?->classify($issued) ?? PremiumTimeContract::STATE_NONE;
        $validUntil = $issued + PremiumTimeContract::MAX_AUTHORITY_LEASE_SECONDS;
        if ($entitlement !== null
            && in_array($state, [PremiumTimeContract::STATE_ACTIVE, PremiumTimeContract::STATE_NOT_YET_EFFECTIVE], true)) {
            $validUntil = min($validUntil, $entitlement->effectiveUntil);
        }
        $refreshAfter = $issued + intdiv(2 * ($validUntil - $issued), 3);

        return [
            'schema' => PremiumTimeContract::SNAPSHOT_SCHEMA,
            'producer_revision' => $producerRevision,
            'producer_profile' => PremiumTimeContract::PRODUCER_PROFILE,
            'nonce' => $nonce,
            'account_id' => $accountId,
            'product_id' => PremiumTimeContract::PRODUCT_ID,
            'product_version' => PremiumTimeContract::PRODUCT_VERSION,
            'entitlement_id' => $entitlement?->id,
            'entitlement_state' => $state,
            'lifecycle_revision' => $entitlement === null ? 0 : $entitlement->lifecycleRevision,
            'authority_revision' => $authorityRevision,
            'effective_from' => $entitlement === null ? null : PremiumTimeContract::formatTime($entitlement->effectiveFrom),
            'effective_until' => $entitlement === null ? null : PremiumTimeContract::formatTime($entitlement->effectiveUntil),
            'authority_issued_at' => PremiumTimeContract::formatTime($issued),
            'authority_valid_until' => PremiumTimeContract::formatTime($validUntil),
            'refresh_after' => PremiumTimeContract::formatTime($refreshAfter),
        ];
    }
}
