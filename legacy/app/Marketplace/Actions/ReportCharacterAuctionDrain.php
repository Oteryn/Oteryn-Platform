<?php

namespace App\Marketplace\Actions;

use App\Marketplace\Models\CharacterAuction;

/**
 * Read-only drain report for the Bazaar freeze: counts every auction that has
 * not reached a terminal status. The Bazaar is drained when none are left.
 */
final class ReportCharacterAuctionDrain
{
    /** @var list<string> */
    public const NON_TERMINAL_STATUSES = [
        CharacterAuction::STATUS_ESCROW_PENDING,
        CharacterAuction::STATUS_ACTIVE,
        CharacterAuction::STATUS_SETTLEMENT_PENDING,
        CharacterAuction::STATUS_CANCEL_PENDING,
        CharacterAuction::STATUS_RECOVERY_REQUIRED,
    ];

    /**
     * @return array{
     *     frozen: bool,
     *     drained: bool,
     *     non_terminal: int,
     *     by_status: array<string, int>,
     *     latest_ends_at: string|null
     * }
     */
    public function run(): array
    {
        $byStatus = array_fill_keys(self::NON_TERMINAL_STATUSES, 0);

        $counts = CharacterAuction::query()
            ->whereIn('status', self::NON_TERMINAL_STATUSES)
            ->selectRaw('status, count(*) as total')
            ->groupBy('status')
            ->pluck('total', 'status');

        foreach ($counts as $status => $total) {
            $byStatus[(string) $status] = is_numeric($total) ? (int) $total : 0;
        }

        $nonTerminal = array_sum($byStatus);
        $latestEndsAt = CharacterAuction::query()
            ->where('status', CharacterAuction::STATUS_ACTIVE)
            ->max('ends_at');

        return [
            'frozen' => config('marketplace.listing_frozen', true) !== false,
            'drained' => $nonTerminal === 0,
            'non_terminal' => $nonTerminal,
            'by_status' => $byStatus,
            'latest_ends_at' => is_string($latestEndsAt) ? $latestEndsAt : null,
        ];
    }
}
