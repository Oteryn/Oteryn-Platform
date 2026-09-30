<?php

namespace App\GameAuth\NativeRuntimeStatus;

use Illuminate\Support\Facades\DB;

/**
 * Acceptance of one `ReportScopeAssignmentV1` (Game producer §5, §6; login contract §7.2). The ownership
 * authority may report only scopes on its list (else 401), and only for a `node_identity` that is a
 * runtime-status identity configured for that scope (else 409). Per scope (assignment_epoch,
 * ownership_generation) only moves forward and no epoch below the highest epoch seen is stored: lower is
 * `superseded`; an equal key with identical content is an idempotent replay (`accepted`); an equal key
 * with other content is a conflict (409) that changes nothing. A higher epoch raises the highest epoch
 * under the exclusive epoch lock, which invalidates every lower-epoch assignment and runtime report.
 */
final class NativeScopeAssignmentIngestion
{
    private const CONFLICT = 'conflict';

    public function __construct(private readonly NativeRuntimeStatusReadModel $readModel) {}

    /** @return 'accepted'|'superseded' */
    public function ingest(NativeScopeAssignmentSettings $settings, NativeRuntimeStatusSettings $runtime, string $identity, NativeScopeAssignmentReport $report): string
    {
        if (! $settings->allows($identity, $report->worldId, $report->channelId)) {
            throw new NativeRuntimeStatusRefused(401);
        }
        if (! $runtime->allows($report->nodeIdentity, $report->worldId, $report->channelId)) {
            throw new NativeRuntimeStatusRefused(409);
        }

        $result = DB::transaction(function () use ($report): string {
            $highest = $this->readModel->lockEpoch(exclusive: true);
            if ($highest !== null && NativeRuntimeStatusReport::compare($report->assignmentEpoch, $highest) < 0) {
                return 'superseded';
            }

            $scope = ['world_id' => $report->worldId, 'channel_id' => $report->channelId];
            $assignments = DB::table('native_scope_assignments');
            $current = (clone $assignments)->where($scope)->lockForUpdate()->first();
            $row = [
                'assignment_epoch' => $report->assignmentEpoch,
                'ownership_generation' => $report->ownershipGeneration,
                'node_identity' => $report->nodeIdentity,
                'assigned_at' => $report->assignedAt,
                'updated_at' => now(),
            ];
            if ($current === null) {
                $assignments->insert($scope + $row + ['created_at' => now()]);

                return 'accepted';
            }

            $order = NativeRuntimeStatusReport::compare($report->assignmentEpoch, NativeRuntimeStatusRow::string($current, 'assignment_epoch'))
                ?: NativeRuntimeStatusReport::compare($report->ownershipGeneration, NativeRuntimeStatusRow::string($current, 'ownership_generation'));
            if ($order > 0) {
                (clone $assignments)->where($scope)->update($row);

                return 'accepted';
            }
            if ($order < 0) {
                return 'superseded';
            }

            return hash_equals(NativeRuntimeStatusRow::string($current, 'node_identity'), $report->nodeIdentity)
                && NativeRuntimeStatusRow::int($current, 'assigned_at') === $report->assignedAt
                ? 'accepted'
                : self::CONFLICT;
        });

        return match ($result) {
            'accepted' => 'accepted',
            'superseded' => 'superseded',
            default => throw new NativeRuntimeStatusRefused(409),
        };
    }
}
