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
     * @return list<NativeAccountCharacterSummary>|null null means unavailable/stale/invalid, [] is an authoritative empty snapshot.
     */
    public function forAccount(string $accountId, int $now): ?array
    {
        $settings = NativeAccountCharactersSettings::current();
        if ($settings === null || $this->feedState($settings, $now) !== self::LIVE) {
            return null;
        }

        $state = DB::table('native_account_character_projection_state')->where('id', 1)->first();
        $snapshot = DB::table('native_account_character_snapshots')->where('account_id', $accountId)->first();
        if ($state === null || $snapshot === null
            || $this->bool($snapshot, 'invalid')
            || ! hash_equals($this->string($state, 'highest_epoch'), $this->string($snapshot, 'projection_epoch'))) {
            return null;
        }

        return DB::table('native_account_character_rows')
            ->where('account_id', $accountId)
            ->orderBy('character_id')
            ->get()
            ->map(fn (object $row): NativeAccountCharacterSummary => new NativeAccountCharacterSummary(
                $this->string($row, 'character_id'),
                $this->string($row, 'world_id'),
                $this->string($row, 'name'),
                $this->string($row, 'availability'),
            ))
            ->all();
    }

    public function feedEvidence(int $now): string
    {
        $settings = NativeAccountCharactersSettings::current();

        return $settings === null ? self::UNAVAILABLE : $this->feedState($settings, $now);
    }

    private function feedState(NativeAccountCharactersSettings $settings, int $now): string
    {
        $row = DB::table('native_account_character_projection_state')->where('id', 1)->first();
        if ($row === null) {
            return self::UNAVAILABLE;
        }

        $highest = $this->nullableString($row, 'highest_epoch');
        $watermarkEpoch = $this->nullableString($row, 'watermark_epoch');
        $completeThrough = $this->nullableInt($row, 'complete_through');
        if ($highest === null || $watermarkEpoch === null || $completeThrough === null || ! hash_equals($highest, $watermarkEpoch)) {
            return self::STALE;
        }

        return $now - $completeThrough + $settings->clockUncertaintySeconds <= $settings->freshnessSeconds
            ? self::LIVE
            : self::STALE;
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
