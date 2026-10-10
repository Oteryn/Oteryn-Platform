<?php

namespace App\GameAuth\NativeRuntimeStatus;

use Illuminate\Support\Facades\DB;
use UnexpectedValueException;

/**
 * Platform consumer evidence state for one native scope (runtime status projection contract, login
 * contract §7.2). Ownership is re-checked on every read: a report routes only while its
 * (assignment_epoch, scope_ownership_generation, identity) equal the scope's latest assignment and that
 * assignment is in the highest epoch seen. `stale`, `unavailable` and `invalid` are never `offline`.
 */
final class NativeRuntimeStatusReadModel
{
    public const FRESH = 'fresh';

    public const STALE = 'stale';

    public const UNAVAILABLE = 'unavailable';

    public const INVALID = 'invalid';

    public function evidence(string $worldId, string $channelId, int $now): string
    {
        return $this->read($worldId, $channelId, $now)[0];
    }

    /**
     * Public-safe evidence for LiveOps/public projections. Invalid/unavailable reports expose no claimed
     * readiness or observation time; stale evidence may retain its last-known ready flag with stale state.
     */
    public function publicEvidence(string $worldId, string $channelId, int $now): NativeRuntimePublicEvidence
    {
        [$state, $report] = $this->read($worldId, $channelId, $now);
        $accepted = $state === self::FRESH || $state === self::STALE;

        return new NativeRuntimePublicEvidence(
            state: $state,
            ready: $accepted && $report !== null ? NativeRuntimeStatusRow::bool($report, 'ready') : null,
            observedAt: $accepted && $report !== null ? NativeRuntimeStatusRow::int($report, 'observed_at') : null,
        );
    }

    /** The report when it is fresh and `ready = true`; otherwise null and the scope routes nowhere. */
    public function routable(string $worldId, string $channelId, int $now): ?NativeRuntimeStatusRecord
    {
        [$state, $report] = $this->read($worldId, $channelId, $now);
        if ($state !== self::FRESH || $report === null || ! NativeRuntimeStatusRow::bool($report, 'ready')) {
            return null;
        }
        $string = static fn (string $column): string => NativeRuntimeStatusRow::string($report, $column);

        return new NativeRuntimeStatusRecord(
            $worldId,
            $channelId,
            $string('route_revision'),
            NativeRuntimeStatusRow::int($report, 'protocol_major'),
            NativeRuntimeStatusRow::int($report, 'transport_profile'),
            $string('scope_ownership_generation'),
            $string('runtime_observation_revision'),
            $string('ruleset_revision'),
            $string('content_revision'),
            $string('map_revision'),
            $string('world_policy_revision'),
            $string('offer_revision'),
            NativeRuntimeStatusRow::int($report, 'observed_at'),
        );
    }

    /** Highest assignment_epoch any scope has; a lower epoch is invalid everywhere (§7.2 restore reset). */
    public function highestEpoch(): ?string
    {
        $highest = null;
        foreach (DB::table('native_scope_assignments')->distinct()->pluck('assignment_epoch') as $epoch) {
            if (is_string($epoch) && ($highest === null || NativeRuntimeStatusReport::compare($epoch, $highest) > 0)) {
                $highest = $epoch;
            }
        }

        return $highest;
    }

    /**
     * Inside a transaction: takes the epoch lock (exclusive to raise the epoch, shared to rely on it) and
     * returns the highest epoch under it. Lock order everywhere: epoch lock, then the scope's assignment
     * row, then its runtime report row.
     */
    public function lockEpoch(bool $exclusive): ?string
    {
        $lock = DB::table('native_assignment_epoch_locks')->where('id', 1);
        if (($exclusive ? $lock->lockForUpdate() : $lock->sharedLock())->first() === null) {
            throw new UnexpectedValueException('Native assignment epoch lock row is missing.');
        }

        return $this->highestEpoch();
    }

    /** @return array{0: string, 1: object|null} */
    private function read(string $worldId, string $channelId, int $now): array
    {
        $settings = NativeRuntimeStatusSettings::current();
        $report = DB::table('native_runtime_status_reports')
            ->where('world_id', $worldId)->where('channel_id', $channelId)->first();
        if ($settings === null || $report === null) {
            return [self::UNAVAILABLE, null];
        }
        $assignment = DB::table('native_scope_assignments')
            ->where('world_id', $worldId)->where('channel_id', $channelId)->first();
        $observedAt = NativeRuntimeStatusRow::int($report, 'observed_at');
        if (NativeRuntimeStatusRow::bool($report, 'invalid')
            || $assignment === null
            || NativeRuntimeStatusRow::string($assignment, 'assignment_epoch') !== $this->highestEpoch()
            || NativeRuntimeStatusRow::string($report, 'assignment_epoch') !== NativeRuntimeStatusRow::string($assignment, 'assignment_epoch')
            || NativeRuntimeStatusRow::string($report, 'scope_ownership_generation') !== NativeRuntimeStatusRow::string($assignment, 'ownership_generation')
            || NativeRuntimeStatusRow::string($report, 'node_identity') !== NativeRuntimeStatusRow::string($assignment, 'node_identity')
            || $observedAt > $now + $settings->clockUncertaintySeconds) {
            return [self::INVALID, $report];
        }

        return [
            $now - $observedAt + $settings->clockUncertaintySeconds <= $settings->freshnessSeconds ? self::FRESH : self::STALE,
            $report,
        ];
    }
}
