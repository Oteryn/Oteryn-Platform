<?php

namespace App\Support\Retention;

use App\Support\Models\EnforcementRecord;
use App\Support\Models\PlayerReport;
use App\Support\Models\SupportTicket;
use App\Support\SupportConfiguration;
use Illuminate\Support\Facades\DB;

final class PruneSupportRetention
{
    /** @return array{tickets_deleted: int, reports_deleted: int, enforcement_anonymized: int} */
    public function execute(bool $dryRun = false): array
    {
        $ticketCutoff = now()->subDays(SupportConfiguration::positiveInteger('support.tickets.retention_days_after_close', 730));
        $reportCutoff = now()->subDays(SupportConfiguration::positiveInteger('support.reports.retention_days_after_close', 730));
        $enforcementCutoff = now()->subDays(SupportConfiguration::positiveInteger('support.enforcement.retention_days_after_expiry', 1095));

        $ticketQuery = SupportTicket::query()
            ->whereIn('status', [SupportTicket::STATUS_RESOLVED, SupportTicket::STATUS_CLOSED])
            ->whereNotNull('closed_at')
            ->where('closed_at', '<', $ticketCutoff);
        $reportQuery = PlayerReport::query()
            ->whereIn('status', [PlayerReport::STATUS_ACTIONED, PlayerReport::STATUS_REJECTED, PlayerReport::STATUS_CLOSED])
            ->whereNotNull('processed_at')
            ->where('processed_at', '<', $reportCutoff);
        $enforcementQuery = EnforcementRecord::query()
            ->whereIn('status', [EnforcementRecord::STATUS_EXPIRED, EnforcementRecord::STATUS_REVOKED])
            ->where(function ($query) use ($enforcementCutoff): void {
                $query->where('expires_at', '<', $enforcementCutoff)
                    ->orWhere(function ($query) use ($enforcementCutoff): void {
                        $query->whereNull('expires_at')->where('updated_at', '<', $enforcementCutoff);
                    });
            })
            ->where(function ($query): void {
                $query->whereNotNull('moderator_notes')->orWhereNotNull('appeal_message');
            });

        $counts = [
            'tickets_deleted' => $ticketQuery->count(),
            'reports_deleted' => $reportQuery->count(),
            'enforcement_anonymized' => $enforcementQuery->count(),
        ];

        if ($dryRun) {
            return $counts;
        }

        return DB::transaction(function () use ($ticketQuery, $reportQuery, $enforcementQuery, $counts): array {
            $ticketQuery->delete();
            $reportQuery->delete();
            $enforcementQuery->update([
                'moderator_notes' => null,
                'appeal_message' => null,
                'updated_at' => now(),
            ]);

            return $counts;
        }, 3);
    }
}
