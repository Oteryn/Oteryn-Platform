<?php

namespace App\CharacterAuthority\Restore;

use App\CharacterAuthority\CharacterCommandIntentRow;
use App\CharacterAuthority\CharacterCommandRefused;
use App\CharacterAuthority\CharacterCommandStoredReceipt;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterCommandWire;
use Illuminate\Database\Query\Builder;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use stdClass;

/**
 * Platform side of a Game restore (Game OPERABILITY §3 ruling 10, commands contract §5.4).
 *
 * On a new notice with restored generation H, one transaction records the notice and reopens as
 * PENDING every intent whose current receipt carries `recovery_generation` ≥ H: the receipt is kept
 * as superseded history and the intent is listed to Game again under its unchanged operation
 * identity. Selection uses only the Game-issued generation, never a Platform time, `decided_at` or
 * T. Platform acknowledges, by answering ACKNOWLEDGED to the same notice, once every operation the
 * notice reopened has one new receipt. There is no cross-database transaction: Game answers each
 * reopened operation from its own restored store through the unchanged command routes.
 */
final class GameRestoreReconciler
{
    public const NOTICES = 'character_restore_notices';

    public const REOPENS = 'character_restore_reopens';

    public const LOCKS = 'character_restore_locks';

    /**
     * Inside a transaction: the restore lock. Notice recording takes it exclusively and receipt
     * publication shared, before any intent lock, so a receipt either commits before a notice
     * selects (and is reopened) or waits until the notice commits (and answers it).
     */
    public static function lock(bool $exclusive): void
    {
        $lock = DB::table(self::LOCKS)->where('id', 1);
        if (($exclusive ? $lock->lockForUpdate() : $lock->sharedLock())->first() === null) {
            throw new CharacterCommandUnavailable('Game restore lock row is missing.');
        }
    }

    public function notice(GameRestoreNoticeSettings $settings, string $raw): string
    {
        $notice = GameRestoreNoticeWire::decode($raw);
        if (! hash_equals($settings->sourceAuthority, $notice->sourceAuthority)) {
            throw new CharacterCommandRefused(401, 'Game restore notice source authority is not configured.');
        }

        $row = $this->stored($notice, $raw);
        if ($row === null) {
            try {
                $row = $this->record($notice, $raw);
            } catch (UniqueConstraintViolationException) {
                // A concurrent delivery of the same notice committed first.
                $row = $this->stored($notice, $raw)
                    ?? throw new CharacterCommandRefused(409, 'Game restore notice changed concurrently.');
            }
        }

        return $this->status($row);
    }

    /** The stored notice for this event, null when none; different bytes for one event are a conflict. */
    private function stored(GameRestoreNotice $notice, string $raw): ?stdClass
    {
        $row = DB::table(self::NOTICES)->where('restore_event_id', $notice->restoreEventId)->first();
        if (! $row instanceof stdClass) {
            return null;
        }
        if (! is_string($row->notice_json ?? null) || ! hash_equals($row->notice_json, $raw)) {
            $this->alarm('character_authority.restore_notice_conflict', $notice);

            throw new CharacterCommandRefused(409, 'Game restore notice conflicts with the stored notice.');
        }

        return $row;
    }

    private function record(GameRestoreNotice $notice, string $raw): stdClass
    {
        return DB::transaction(function () use ($notice, $raw): stdClass {
            // Serializes this notice with every receipt publication and every other notice.
            self::lock(true);
            // A concurrent delivery of the same notice committed while this one waited.
            $existing = $this->stored($notice, $raw);
            if ($existing !== null) {
                return $existing;
            }

            $newGeneration = GameRestoreNoticeWire::padGeneration($notice->newRecoveryGeneration);
            $restored = GameRestoreNoticeWire::padGeneration($notice->restoredGeneration);
            // Generations only rise: a notice that does not advance past every earlier one is stale.
            $stale = DB::table(self::NOTICES)->where('new_recovery_generation', '>=', $newGeneration)->exists();
            if ($stale) {
                $this->alarm('character_authority.restore_notice_stale', $notice);

                throw new CharacterCommandRefused(409, 'Game restore notice does not advance the recovery generation.');
            }

            $now = now();
            $noticeId = DB::table(self::NOTICES)->insertGetId([
                'restore_event_id' => $notice->restoreEventId,
                'source_authority' => $notice->sourceAuthority,
                'restored_generation' => $restored,
                'new_recovery_generation' => $newGeneration,
                'notice_json' => $raw,
                'notice_sha256' => hash('sha256', $raw),
                'reopened_count' => 0,
                'received_at' => $now,
                'acknowledged_at' => null,
            ]);

            // No receipt commits while the restore lock is held, so both selections see one state.
            $reopened = $this->carryOpen($noticeId, $restored, $now) + $this->reopenTerminal($noticeId, $restored, $now);
            DB::table(self::NOTICES)->where('id', $noticeId)->update(['reopened_count' => $reopened]);

            Log::warning('character_authority.restore_notice_recorded', [
                'restore_event_id' => $notice->restoreEventId,
                'restored_generation' => $notice->restoredGeneration,
                'new_recovery_generation' => $notice->newRecoveryGeneration,
                'reopened' => $reopened,
            ]);

            $row = DB::table(self::NOTICES)->where('id', $noticeId)->first();
            if (! $row instanceof stdClass) {
                throw new CharacterCommandRefused(503, 'Game restore notice was not recorded.');
            }

            return $row;
        }, 3);
    }

    /** Reopens every TERMINAL intent whose current receipt has generation ≥ H. */
    private function reopenTerminal(int $noticeId, string $restored, mixed $now): int
    {
        $intentIds = DB::table(CharacterCommandIntentRow::TABLE.' as intent')
            ->join(CharacterCommandStoredReceipt::TABLE.' as receipt', 'receipt.id', '=', 'intent.current_receipt_id')
            ->where('intent.issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('intent.state', CharacterCommandIntentRow::TERMINAL)
            ->where('receipt.recovery_generation', '>=', $restored)
            ->orderBy('intent.id')
            ->pluck('intent.id');

        $reopened = 0;
        foreach ($intentIds as $intentId) {
            // The restore lock already excludes receipts; the intent lock also orders client reads.
            $intent = DB::table(CharacterCommandIntentRow::TABLE)->where('id', $intentId)->lockForUpdate()->first();
            if (! $intent instanceof stdClass) {
                continue;
            }
            $current = CharacterCommandIntentRow::fromRow($intent);
            if ($current->state !== CharacterCommandIntentRow::TERMINAL || $current->currentReceiptId === null) {
                continue;
            }
            $receipt = DB::table(CharacterCommandStoredReceipt::TABLE)->where('id', $current->currentReceiptId)->first();
            if (! $receipt instanceof stdClass || ! is_string($receipt->recovery_generation ?? null)
                || strcmp($receipt->recovery_generation, $restored) < 0) {
                continue;
            }

            DB::table(CharacterCommandStoredReceipt::TABLE)->where('id', $current->currentReceiptId)->update(['superseded_at' => $now]);
            DB::table(CharacterCommandIntentRow::TABLE)->where('id', $current->id)->update([
                'state' => CharacterCommandIntentRow::PENDING,
                'current_receipt_id' => null,
                'updated_at' => $now,
            ]);
            DB::table(self::REOPENS)->insert([
                'notice_id' => $noticeId,
                'intent_id' => $current->id,
                'superseded_receipt_id' => $current->currentReceiptId,
                'new_receipt_id' => null,
                'created_at' => $now,
            ]);
            Log::info('character_authority.restore_intent_reopened', [
                'operation_id' => $current->operationId,
                'command' => $current->command,
            ]);
            $reopened++;
        }

        return $reopened;
    }

    /**
     * An intent an earlier notice reopened that still waits for its new receipt, whose superseded
     * receipt also has generation ≥ H, is lost to this restore too: this notice waits for it as well.
     */
    private function carryOpen(int $noticeId, string $restored, mixed $now): int
    {
        $intentIds = $this->openReopens($noticeId, $restored)->pluck('reopen.intent_id')->unique()->sort()->values();

        $carried = 0;
        foreach ($intentIds as $intentId) {
            // An intent whose earlier reopen was answered is TERMINAL again; reopenTerminal() decides it.
            $intent = DB::table(CharacterCommandIntentRow::TABLE)->where('id', $intentId)->lockForUpdate()->first();
            if (! $intent instanceof stdClass || CharacterCommandIntentRow::fromRow($intent)->state !== CharacterCommandIntentRow::PENDING) {
                continue;
            }
            $open = $this->openReopens($noticeId, $restored)
                ->where('reopen.intent_id', $intentId)
                ->orderBy('reopen.id')
                ->first(['reopen.superseded_receipt_id']);
            if (! $open instanceof stdClass) {
                continue;
            }
            DB::table(self::REOPENS)->insert([
                'notice_id' => $noticeId,
                'intent_id' => $intentId,
                'superseded_receipt_id' => $open->superseded_receipt_id,
                'new_receipt_id' => null,
                'created_at' => $now,
            ]);
            $carried++;
        }

        return $carried;
    }

    /** Reopen rows of earlier notices still waiting for a new receipt whose superseded receipt has generation ≥ H. */
    private function openReopens(int $noticeId, string $restored): Builder
    {
        return DB::table(self::REOPENS.' as reopen')
            ->join(CharacterCommandStoredReceipt::TABLE.' as receipt', 'receipt.id', '=', 'reopen.superseded_receipt_id')
            ->whereNull('reopen.new_receipt_id')
            ->where('reopen.notice_id', '!=', $noticeId)
            ->where('receipt.recovery_generation', '>=', $restored);
    }

    private function status(stdClass $notice): string
    {
        $eventId = $notice->restore_event_id ?? null;
        $reopened = $notice->reopened_count ?? null;
        if (! is_string($eventId) || ! is_numeric($reopened)) {
            throw new CharacterCommandRefused(503, 'Stored Game restore notice is unreadable.');
        }
        $reopened = (int) $reopened;
        $outstanding = DB::table(self::REOPENS)->where('notice_id', $notice->id)->whereNull('new_receipt_id')->count();

        if ($outstanding === 0 && $notice->acknowledged_at === null) {
            $updated = DB::table(self::NOTICES)->where('id', $notice->id)->whereNull('acknowledged_at')->update(['acknowledged_at' => now()]);
            if ($updated === 1) {
                Log::warning('character_authority.restore_notice_acknowledged', ['restore_event_id' => $eventId, 'reopened' => $reopened]);
            }
        }

        return GameRestoreNoticeWire::encodeStatus($eventId, $reopened, $outstanding);
    }

    private function alarm(string $event, GameRestoreNotice $notice): void
    {
        Log::error($event, [
            'restore_event_id' => $notice->restoreEventId,
            'restored_generation' => $notice->restoredGeneration,
            'new_recovery_generation' => $notice->newRecoveryGeneration,
        ]);
    }
}
