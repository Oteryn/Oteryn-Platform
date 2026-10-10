<?php

namespace App\GameAuth\NativeAccountCharacters;

use Illuminate\Support\Facades\DB;
use UnexpectedValueException;

final class NativeAccountCharactersReadModel
{
    public const LIVE = 'live';

    public const STALE = 'stale';

    public const UNAVAILABLE = 'unavailable';

    /**
     * Compatibility helper for callers that need only a usable snapshot.
     *
     * @return list<NativeAccountCharacterSummary>|null
     */
    public function forAccount(string $accountId, int $now): ?array
    {
        $view = $this->viewForAccount($accountId, $now);

        return $view->state === NativeAccountCharactersAccountView::READY ? $view->characters : null;
    }

    public function viewForAccount(string $accountId, int $now, bool $lock = false): NativeAccountCharactersAccountView
    {
        $settings = NativeAccountCharactersSettings::current();
        if ($settings === null) {
            return new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::UNAVAILABLE);
        }

        [$feed, $highestEpoch] = $this->projectionState($settings, $now, $lock);
        if ($feed === self::UNAVAILABLE) {
            return new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::UNAVAILABLE);
        }
        if ($feed !== self::LIVE || $highestEpoch === null) {
            return new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::STALE);
        }

        $snapshotQuery = DB::table('native_account_character_snapshots')->where('account_id', $accountId);
        if ($lock) {
            $snapshotQuery->sharedLock();
        }
        $snapshot = $snapshotQuery->first();
        if ($snapshot === null) {
            return new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::MISSING);
        }
        if ($this->bool($snapshot, 'invalid')
            || ! hash_equals($highestEpoch, $this->string($snapshot, 'projection_epoch'))) {
            return new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::INVALID);
        }

        // Under InnoDB REPEATABLE READ a plain read uses the transaction's first consistent view, which can
        // predate the locked snapshot row; the rows must be a locking read too so they match that snapshot.
        // (With MariaDB innodb_snapshot_isolation the locked snapshot read itself fails with ER_CHECKREAD.)
        $rowsQuery = DB::table('native_account_character_rows')
            ->where('account_id', $accountId)
            ->orderBy('character_id');
        if ($lock) {
            $rowsQuery->sharedLock();
        }
        $characters = array_values($rowsQuery
            ->get()
            ->map(fn (object $row): NativeAccountCharacterSummary => new NativeAccountCharacterSummary(
                $this->string($row, 'character_id'),
                $this->string($row, 'world_id'),
                $this->string($row, 'name'),
                $this->string($row, 'availability'),
            ))
            ->all());

        return new NativeAccountCharactersAccountView(NativeAccountCharactersAccountView::READY, $characters);
    }

    public function feedEvidence(int $now): string
    {
        $settings = NativeAccountCharactersSettings::current();
        if ($settings === null) {
            return self::UNAVAILABLE;
        }

        return $this->projectionState($settings, $now)[0];
    }

    /** @return array{0:string,1:string|null} */
    private function projectionState(NativeAccountCharactersSettings $settings, int $now, bool $lock = false): array
    {
        if ($lock && DB::transactionLevel() < 1) {
            throw new UnexpectedValueException('Locking native account-character reads require a transaction.');
        }

        $query = DB::table('native_account_character_projection_state')->where('id', 1);
        if ($lock) {
            $query->sharedLock();
        }
        $row = $query->first();
        if ($row === null) {
            return [self::UNAVAILABLE, null];
        }

        $highest = $this->nullableString($row, 'highest_epoch');
        $watermarkEpoch = $this->nullableString($row, 'watermark_epoch');
        $completeThrough = $this->nullableInt($row, 'complete_through');
        if ($highest === null || $watermarkEpoch === null || $completeThrough === null || ! hash_equals($highest, $watermarkEpoch)) {
            return [self::STALE, $highest];
        }
        if ($completeThrough > $now + $settings->clockUncertaintySeconds) {
            return [self::STALE, $highest];
        }

        $feed = $now - $completeThrough + $settings->clockUncertaintySeconds <= $settings->freshnessSeconds
            ? self::LIVE
            : self::STALE;

        return [$feed, $highest];
    }

    private function string(object $row, string $column): string
    {
        $value = get_object_vars($row)[$column] ?? null;
        if (! is_string($value)) {
            throw new UnexpectedValueException('Native account-character column is not a string.');
        }

        return $value;
    }

    private function nullableString(object $row, string $column): ?string
    {
        $value = get_object_vars($row)[$column] ?? null;
        if ($value !== null && ! is_string($value)) {
            throw new UnexpectedValueException('Native account-character nullable column is invalid.');
        }

        return $value;
    }

    private function nullableInt(object $row, string $column): ?int
    {
        $value = get_object_vars($row)[$column] ?? null;
        if ($value === null) {
            return null;
        }
        if (is_string($value) && ctype_digit($value)) {
            $value = (int) $value;
        }
        if (! is_int($value)) {
            throw new UnexpectedValueException('Native account-character timestamp column is invalid.');
        }

        return $value;
    }

    private function bool(object $row, string $column): bool
    {
        $value = get_object_vars($row)[$column] ?? null;
        if ($value === true || $value === 1 || $value === '1') {
            return true;
        }
        if ($value === false || $value === 0 || $value === '0') {
            return false;
        }

        throw new UnexpectedValueException('Native account-character boolean column is invalid.');
    }
}
