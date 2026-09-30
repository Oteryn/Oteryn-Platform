<?php

namespace App\GameAuth\NativeRuntimeStatus;

use Illuminate\Support\Facades\DB;

/**
 * Acceptance of one `ReportRuntimeStatusV1` (Game producer §7, login contract §7.2). The report is
 * accepted only for a scope its identity may serve (else 401) and only when its
 * (assignment_epoch, scope_ownership_generation) and identity equal the scope's latest assignment in
 * the highest epoch (else 409). Per scope the key (assignment_epoch, scope_ownership_generation,
 * source_revision) only moves forward: lower is `superseded`; an equal key with other content marks
 * the scope invalid (409) until a higher key; an equal key with identical content refreshes observed_at.
 */
final class NativeRuntimeStatusIngestion
{
    private const CONFLICT = 'conflict';

    public function __construct(private readonly NativeRuntimeStatusReadModel $readModel) {}

    /** @return 'accepted'|'refreshed'|'superseded' */
    public function ingest(NativeRuntimeStatusSettings $settings, string $identity, NativeRuntimeStatusReport $report, int $now): string
    {
        if (! $settings->allows($identity, $report->worldId, $report->channelId)) {
            throw new NativeRuntimeStatusRefused(401);
        }
        if ($report->observedAt > $now + $settings->clockUncertaintySeconds) {
            throw new NativeRuntimeStatusRefused(400);
        }

        $result = DB::transaction(function () use ($identity, $report): string {
            $scope = ['world_id' => $report->worldId, 'channel_id' => $report->channelId];
            $assignment = DB::table('native_scope_assignments')->where($scope)->lockForUpdate()->first();
            if ($assignment === null
                || NativeRuntimeStatusRow::string($assignment, 'assignment_epoch') !== $this->readModel->highestEpoch()
                || NativeRuntimeStatusRow::string($assignment, 'assignment_epoch') !== $report->assignmentEpoch
                || NativeRuntimeStatusRow::string($assignment, 'ownership_generation') !== $report->scopeOwnershipGeneration
                || ! hash_equals(NativeRuntimeStatusRow::string($assignment, 'node_identity'), $identity)) {
                return self::CONFLICT;
            }

            $reports = DB::table('native_runtime_status_reports');
            $current = (clone $reports)->where($scope)->lockForUpdate()->first();
            $digest = $report->contentDigest();
            $row = $report->content + [
                'observed_at' => $report->observedAt,
                'node_identity' => $identity,
                'content_digest' => $digest,
                'invalid' => false,
                'updated_at' => now(),
            ];
            if ($current === null) {
                $reports->insert($row + ['created_at' => now()]);

                return 'accepted';
            }

            $order = $this->order($report, $current);
            if ($order < 0) {
                return 'superseded';
            }
            if ($order > 0) {
                (clone $reports)->where($scope)->update($row);

                return 'accepted';
            }
            if (NativeRuntimeStatusRow::bool($current, 'invalid')
                || ! hash_equals(NativeRuntimeStatusRow::string($current, 'content_digest'), $digest)) {
                (clone $reports)->where($scope)->update(['invalid' => true, 'updated_at' => now()]);

                return self::CONFLICT;
            }
            if ($report->observedAt <= NativeRuntimeStatusRow::int($current, 'observed_at')) {
                return 'superseded';
            }
            (clone $reports)->where($scope)->update(['observed_at' => $report->observedAt, 'updated_at' => now()]);

            return 'refreshed';
        });

        return match ($result) {
            'accepted' => 'accepted',
            'refreshed' => 'refreshed',
            'superseded' => 'superseded',
            default => throw new NativeRuntimeStatusRefused(409),
        };
    }

    private function order(NativeRuntimeStatusReport $report, object $current): int
    {
        return NativeRuntimeStatusReport::compare($report->assignmentEpoch, NativeRuntimeStatusRow::string($current, 'assignment_epoch'))
            ?: NativeRuntimeStatusReport::compare($report->scopeOwnershipGeneration, NativeRuntimeStatusRow::string($current, 'scope_ownership_generation'))
            ?: NativeRuntimeStatusReport::compare($report->sourceRevision, NativeRuntimeStatusRow::string($current, 'source_revision'));
    }
}
