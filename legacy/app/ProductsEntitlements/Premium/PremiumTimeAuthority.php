<?php

namespace App\ProductsEntitlements\Premium;

use Illuminate\Support\Facades\DB;

/**
 * The per-account authority row. Grants, revocations and snapshot issuance all lock it first, so they serialize
 * per account and `authority_revision` is ordered with lifecycle changes (contract 6.4). Call inside a transaction.
 */
final class PremiumTimeAuthority
{
    public function lock(int $identityId): int
    {
        DB::table('premium_time_authority')->insertOrIgnore([
            'identity_id' => $identityId,
            'authority_revision' => 0,
            'updated_at' => now(),
        ]);

        $revision = PremiumTimeContract::databaseInteger(DB::table('premium_time_authority')
            ->where('identity_id', $identityId)
            ->lockForUpdate()
            ->value('authority_revision'));
        if ($revision === null || $revision > PremiumTimeContract::MAX_REVISION) {
            throw new PremiumSnapshotUnavailable('Premium time authority row is unavailable.');
        }

        return $revision;
    }

    /** Allocate the next authority revision for a snapshot. The caller holds the row lock. */
    public function advance(int $identityId, int $current): int
    {
        $next = $current + 1;
        if ($next > PremiumTimeContract::MAX_REVISION) {
            throw new PremiumSnapshotUnavailable('Premium time authority revision is exhausted.');
        }

        $updated = DB::table('premium_time_authority')
            ->where('identity_id', $identityId)
            ->where('authority_revision', $current)
            ->update(['authority_revision' => $next, 'updated_at' => now()]);
        if ($updated !== 1) {
            throw new PremiumSnapshotUnavailable('Premium time authority revision could not be advanced.');
        }

        return $next;
    }
}
