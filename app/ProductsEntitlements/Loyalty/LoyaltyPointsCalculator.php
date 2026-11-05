<?php

namespace App\ProductsEntitlements\Loyalty;

use App\ProductsEntitlements\Premium\PremiumSnapshotUnavailable;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use Illuminate\Support\Facades\DB;

/** Computes whole consumed Premium days from the append-only lifecycle event history. */
final class LoyaltyPointsCalculator
{
    public function calculate(int $identityId, int $now): int
    {
        $events = DB::table('premium_time_entitlement_events')
            ->where('identity_id', $identityId)
            ->orderBy('entitlement_id')
            ->orderBy('effective_from')
            ->orderBy('lifecycle_revision')
            ->get(['entitlement_id', 'lifecycle_revision', 'effective_from', 'effective_until']);

        /** @var array<string, array{revision:int,from:int,until:int}> $intervals */
        $intervals = [];
        foreach ($events as $event) {
            $revision = PremiumTimeContract::databaseInteger($event->lifecycle_revision);
            $from = PremiumTimeContract::databaseInteger($event->effective_from);
            $until = PremiumTimeContract::databaseInteger($event->effective_until);
            if (! is_string($event->entitlement_id) || $revision === null || $from === null || $until === null) {
                throw new PremiumSnapshotUnavailable('Premium time event history is invalid.');
            }

            $key = $event->entitlement_id."\0".$from;
            if (! isset($intervals[$key]) || $revision > $intervals[$key]['revision']) {
                $intervals[$key] = compact('revision', 'from', 'until');
            }
        }

        $consumed = 0;
        $maximumSeconds = LoyaltyContract::MAX_POINTS * LoyaltyContract::SECONDS_PER_POINT;
        foreach ($intervals as $interval) {
            if ($interval['from'] >= $now) {
                continue;
            }
            $consumed += max(0, min($interval['until'], $now) - $interval['from']);
            if ($consumed >= $maximumSeconds) {
                return LoyaltyContract::MAX_POINTS;
            }
        }

        return intdiv($consumed, LoyaltyContract::SECONDS_PER_POINT);
    }
}
