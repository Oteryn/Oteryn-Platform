<?php

namespace App\ProductsEntitlements\Loyalty;

use App\ProductsEntitlements\Premium\PremiumSnapshotUnavailable;
use App\ProductsEntitlements\Premium\PremiumTimeAuthority;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use Illuminate\Support\Facades\DB;
use JsonException;

/** Issues one account's loyalty projection using the shared Premium authority revision. */
final class LoyaltySnapshotIssuer
{
    public function __construct(
        private readonly PremiumTimeAuthority $authority,
        private readonly LoyaltyPointsCalculator $calculator,
    ) {}

    /** @return array<string, int|string>|null */
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
            $loyaltyRevision = $this->authority->advance($identityId, $current);
            $computed = now()->getTimestamp();

            return [
                'schema' => LoyaltyContract::SNAPSHOT_SCHEMA,
                'producer_revision' => $producerRevision,
                'nonce' => $nonce,
                'account_id' => $accountId,
                'loyalty_points' => $this->calculator->calculate($identityId, $computed),
                'loyalty_revision' => $loyaltyRevision,
                'computed_at' => PremiumTimeContract::formatTime($computed),
            ];
        }, 3);

        try {
            $bytes = strlen(json_encode($snapshot, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));
        } catch (JsonException) {
            throw new PremiumSnapshotUnavailable('Loyalty snapshot could not be encoded.');
        }
        if ($bytes > LoyaltyContract::MAX_RESPONSE_BYTES) {
            throw new PremiumSnapshotUnavailable('Loyalty snapshot exceeds its size bound.');
        }

        return $snapshot;
    }
}
