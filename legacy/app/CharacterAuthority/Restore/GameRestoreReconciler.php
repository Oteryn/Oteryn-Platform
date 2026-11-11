<?php

namespace App\CharacterAuthority\Restore;

use App\CharacterAuthority\CharacterCommandIntentRow;
use App\CharacterAuthority\CharacterCommandRefused;
use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandStoredReceipt;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterCommandWire;
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
 * T. Every operation an earlier notice reopened that still has no new receipt is carried into the
 * new notice too, whatever its superseded receipt's generation. Platform acknowledges, by answering
 * ACKNOWLEDGED to the same notice, once every operation the notice reopened or carried has one new
 * receipt. There is no cross-database transaction: Game answers each
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
            $reopened = $this->carryOpen($noticeId, $now) + $this->reopenTerminal($noticeId, $restored, $now);
            if ($reopened > 0 && CharacterCommandSettings::exchange() === null) {
                // Game could not list or answer what this notice reopens: commit nothing, Game retries.
                $this->alarm('character_authority.restore_notice_exchange_unavailable', $notice);

                throw new CharacterCommandUnavailable('Character command exchange is unavailable for reopened operations.');
            }
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
     * Every intent an earlier notice reopened that still waits for its new receipt is carried into
     * this notice, whatever the generation of the receipt it superseded: that decision was already
     * lost to an earlier restore, and the answer Game may have made since is at or above that
     * notice's new generation, so this restore can lose it too. This notice waits for the same one
     * new receipt, which the receipt route accepts only at or above this notice's new generation.
     */
    private function carryOpen(int $noticeId, mixed $now): int
    {
        // All open rows of one intent name the receipt its first open reopen superseded.
        $open = DB::table(self::REOPENS)
            ->whereNull('new_receipt_id')
            ->where('notice_id', '!=', $noticeId)
            ->orderBy('intent_id')
            ->orderBy('id')
            ->get(['intent_id', 'superseded_receipt_id'])
            ->unique('intent_id');

        $carried = 0;
        foreach ($open as $reopen) {
            // The restore lock excludes receipts, so an intent with an open reopen is still PENDING.
            $intent = DB::table(CharacterCommandIntentRow::TABLE)->where('id', $reopen->intent_id)->lockForUpdate()->first();
            if (! $intent instanceof stdClass || CharacterCommandIntentRow::fromRow($intent)->state !== CharacterCommandIntentRow::PENDING) {
                // An answered intent is TERMINAL again with its reopens closed; reopenTerminal() decides it.
                continue;
            }
            DB::table(self::REOPENS)->insert([
                'notice_id' => $noticeId,
                'intent_id' => $reopen->intent_id,
                'superseded_receipt_id' => $reopen->superseded_receipt_id,
                'new_receipt_id' => null,
                'created_at' => $now,
            ]);
            $carried++;
        }

        return $carried;
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
