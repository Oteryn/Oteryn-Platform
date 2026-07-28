<?php

namespace App\Support\Actions;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Support\Models\PlayerReport;
use App\Support\Notifications\SupportNotificationDeliveryService;
use DomainException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use InvalidArgumentException;

final class ManagePlayerReport
{
    public function __construct(
        private readonly AdminAuditRecorder $audit,
        private readonly SupportNotificationDeliveryService $notifications,
    ) {}

    public function submit(
        Identity $reporter,
        string $requestKey,
        string $reportType,
        string $category,
        string $targetReference,
        ?string $evidenceSummary,
    ): PlayerReport {
        $this->assertTypeAndCategory($reportType, $category);
        $target = trim($targetReference);
        $evidence = trim((string) $evidenceSummary);

        if ($target === '') {
            throw new InvalidArgumentException('A report target is required.');
        }

        return DB::transaction(function () use (
            $reporter,
            $requestKey,
            $reportType,
            $category,
            $target,
            $evidence,
        ): PlayerReport {
            $lockedReporter = Identity::query()->lockForUpdate()->findOrFail($reporter->id);
            abort_if($lockedReporter->isTerminated(), 403);

            $existing = PlayerReport::query()
                ->where('reporter_identity_id', $lockedReporter->id)
                ->where('request_key', $requestKey)
                ->first();
            if ($existing instanceof PlayerReport) {
                return $existing;
            }

            $pendingCount = PlayerReport::query()
                ->where('reporter_identity_id', $lockedReporter->id)
                ->whereIn('status', [PlayerReport::STATUS_SUBMITTED, PlayerReport::STATUS_REVIEWING])
                ->count();
            if ($pendingCount >= (int) config('support.reports.pending_limit_per_identity', 5)) {
                throw new DomainException('The pending report limit has been reached.');
            }

            return PlayerReport::query()->create([
                'public_id' => (string) Str::ulid(),
                'reporter_identity_id' => $lockedReporter->id,
                'request_key' => $requestKey,
                'report_type' => $reportType,
                'category' => $category,
                'target_reference' => $target,
                'evidence_summary' => $evidence === '' ? null : $evidence,
                'status' => PlayerReport::STATUS_SUBMITTED,
                'lock_version' => 1,
            ]);
        }, 3);
    }

    public function moderate(
        Identity $actor,
        PlayerReport $report,
        string $status,
        ?string $publicOutcome,
        ?string $moderatorNotes,
        int $expectedLockVersion,
        string $locale = 'en',
    ): PlayerReport {
        if (! in_array($status, PlayerReport::statuses(), true)) {
            throw new InvalidArgumentException('Unsupported report status.');
        }

        $outcome = trim((string) $publicOutcome);
        $notes = trim((string) $moderatorNotes);
        if (in_array($status, [PlayerReport::STATUS_ACTIONED, PlayerReport::STATUS_REJECTED, PlayerReport::STATUS_CLOSED], true) && $outcome === '') {
            throw new InvalidArgumentException('A public-safe outcome is required for a processed report.');
        }

        $saved = DB::transaction(function () use (
            $actor,
            $report,
            $status,
            $outcome,
            $notes,
            $expectedLockVersion,
        ): PlayerReport {
            $current = PlayerReport::query()->lockForUpdate()->findOrFail($report->id);
            if ($current->lock_version !== $expectedLockVersion) {
                throw new DomainException('This report changed after the page was opened. Reload it before continuing.');
            }

            $this->assertTransition($current->status, $status);

            $current->forceFill([
                'status' => $status,
                'public_outcome' => $outcome === '' ? null : $outcome,
                'moderator_notes' => $notes === '' ? null : $notes,
                'assigned_to' => $actor->id,
                'processed_at' => in_array($status, [PlayerReport::STATUS_ACTIONED, PlayerReport::STATUS_REJECTED, PlayerReport::STATUS_CLOSED], true)
                    ? now()
                    : null,
                'lock_version' => $current->lock_version + 1,
            ])->save();

            $this->audit->record(
                $actor->id,
                'support.report_status_changed',
                'player_report',
                $current->public_id,
                [
                    'report_type' => $current->report_type,
                    'category' => $current->category,
                    'status' => $current->status,
                    'has_public_outcome' => $current->public_outcome !== null,
                    'has_private_notes' => $current->moderator_notes !== null,
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);

        if (in_array($saved->status, [PlayerReport::STATUS_ACTIONED, PlayerReport::STATUS_REJECTED, PlayerReport::STATUS_CLOSED], true)) {
            $reporter = Identity::query()->find($saved->reporter_identity_id);
            if ($reporter instanceof Identity) {
                $this->notifications->queue(
                    $reporter,
                    SupportNotificationDeliveryService::REPORT_OUTCOME,
                    'report',
                    $saved->public_id,
                    $locale,
                );
            }
        }

        return $saved;
    }

    /** @return array<string, list<string>> */
    public static function categories(): array
    {
        return [
            PlayerReport::TYPE_PLAYER => ['harassment', 'cheating', 'name', 'conduct', 'other'],
            PlayerReport::TYPE_CONTENT => ['illegal_content', 'privacy', 'spam', 'misinformation', 'other'],
            PlayerReport::TYPE_GUILD => ['name', 'description', 'harassment', 'impersonation', 'other'],
        ];
    }

    private function assertTypeAndCategory(string $type, string $category): void
    {
        $categories = self::categories();
        if (! isset($categories[$type]) || ! in_array($category, $categories[$type], true)) {
            throw new InvalidArgumentException('Unsupported report type or category.');
        }
    }

    private function assertTransition(string $from, string $to): void
    {
        $allowed = [
            PlayerReport::STATUS_SUBMITTED => [
                PlayerReport::STATUS_SUBMITTED,
                PlayerReport::STATUS_REVIEWING,
                PlayerReport::STATUS_ACTIONED,
                PlayerReport::STATUS_REJECTED,
            ],
            PlayerReport::STATUS_REVIEWING => [
                PlayerReport::STATUS_REVIEWING,
                PlayerReport::STATUS_ACTIONED,
                PlayerReport::STATUS_REJECTED,
            ],
            PlayerReport::STATUS_ACTIONED => [PlayerReport::STATUS_CLOSED],
            PlayerReport::STATUS_REJECTED => [PlayerReport::STATUS_CLOSED],
            PlayerReport::STATUS_CLOSED => [],
        ];

        if (! in_array($to, $allowed[$from] ?? [], true)) {
            throw new DomainException('The requested report status transition is not allowed.');
        }
    }
}
