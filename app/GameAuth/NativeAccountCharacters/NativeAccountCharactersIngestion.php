<?php

namespace App\GameAuth\NativeAccountCharacters;

use Illuminate\Support\Facades\DB;

final class NativeAccountCharactersIngestion
{
    /** @return 'accepted'|'superseded' */
    public function snapshot(NativeAccountCharactersSettings $settings, string $identity, NativeAccountCharactersSnapshot $snapshot, int $now): string
    {
        if (! $settings->allows($identity, $snapshot->sourceAuthority)) {
            throw new NativeAccountCharactersRefused(401);
        }
        if ($snapshot->sourceObservedAt > $now + $settings->clockUncertaintySeconds) {
            throw new NativeAccountCharactersRefused(400);
        }

        return DB::transaction(function () use ($snapshot): string {
            $state = DB::table('native_account_character_projection_state')->where('id', 1)->lockForUpdate()->first();
            if ($state === null) {
                throw new NativeAccountCharactersRefused(503);
            }

            $highest = $this->nullableString($state, 'highest_epoch');
            if ($highest !== null) {
                $epochOrder = NativeAccountCharactersWire::compareUint64($snapshot->projectionEpoch, $highest);
                if ($epochOrder < 0) {
                    return 'superseded';
                }
                if ($epochOrder > 0) {
                    $this->raiseEpoch($snapshot->projectionEpoch);
                }
            } else {
                $this->raiseEpoch($snapshot->projectionEpoch);
            }

            $current = DB::table('native_account_character_snapshots')->where('account_id', $snapshot->accountId)->lockForUpdate()->first();
            if ($current !== null) {
                $epochOrder = NativeAccountCharactersWire::compareUint64($snapshot->projectionEpoch, $this->string($current, 'projection_epoch'));
                $revisionOrder = $epochOrder === 0
                    ? NativeAccountCharactersWire::compareUint64($snapshot->projectionRevision, $this->string($current, 'projection_revision'))
                    : $epochOrder;
                if ($revisionOrder < 0) {
                    return 'superseded';
                }
                if ($revisionOrder === 0) {
                    if (! hash_equals($this->string($current, 'content_digest'), $snapshot->contentDigest)) {
                        DB::table('native_account_character_snapshots')->where('account_id', $snapshot->accountId)->update([
                            'invalid' => true,
                            'updated_at' => now(),
                        ]);
                        throw new NativeAccountCharactersRefused(409);
                    }

                    return 'accepted';
                }
            }

            $values = [
                'source_authority' => $snapshot->sourceAuthority,
                'projection_epoch' => $snapshot->projectionEpoch,
                'projection_revision' => $snapshot->projectionRevision,
                'source_observed_at' => $snapshot->sourceObservedAt,
                'content_digest' => $snapshot->contentDigest,
                'invalid' => false,
                'updated_at' => now(),
            ];
            if ($current === null) {
                DB::table('native_account_character_snapshots')->insert($values + [
                    'account_id' => $snapshot->accountId,
                    'created_at' => now(),
                ]);
            } else {
                DB::table('native_account_character_snapshots')->where('account_id', $snapshot->accountId)->update($values);
            }

            DB::table('native_account_character_rows')->where('account_id', $snapshot->accountId)->delete();
            if ($snapshot->characters !== []) {
                DB::table('native_account_character_rows')->insert(array_map(
                    fn (NativeAccountCharacterSummary $character): array => [
                        'account_id' => $snapshot->accountId,
                        'character_id' => $character->characterId,
                        'world_id' => $character->worldId,
                        'name' => $character->name,
                        'availability' => $character->availability,
                        'created_at' => now(),
                        'updated_at' => now(),
                    ],
                    $snapshot->characters,
                ));
            }

            return 'accepted';
        });
    }

    /** @return 'accepted'|'superseded' */
    public function watermark(NativeAccountCharactersSettings $settings, string $identity, NativeAccountCharactersWatermark $watermark, int $now): string
    {
        if (! $settings->allows($identity, $watermark->sourceAuthority)) {
            throw new NativeAccountCharactersRefused(401);
        }
        if ($watermark->observedAt > $now + $settings->clockUncertaintySeconds
            || $watermark->completeThrough > $now + $settings->clockUncertaintySeconds) {
            throw new NativeAccountCharactersRefused(400);
        }

        return DB::transaction(function () use ($watermark): string {
            $state = DB::table('native_account_character_projection_state')->where('id', 1)->lockForUpdate()->first();
            if ($state === null) {
                throw new NativeAccountCharactersRefused(503);
            }

            $highest = $this->nullableString($state, 'highest_epoch');
            if ($highest !== null) {
                $epochOrder = NativeAccountCharactersWire::compareUint64($watermark->projectionEpoch, $highest);
                if ($epochOrder < 0) {
                    return 'superseded';
                }
                if ($epochOrder > 0) {
                    $this->raiseEpoch($watermark->projectionEpoch);
                    $state = DB::table('native_account_character_projection_state')->where('id', 1)->lockForUpdate()->first();
                }
            } else {
                $this->raiseEpoch($watermark->projectionEpoch);
                $state = DB::table('native_account_character_projection_state')->where('id', 1)->lockForUpdate()->first();
            }
            if ($state === null) {
                throw new NativeAccountCharactersRefused(503);
            }

            $currentEpoch = $this->nullableString($state, 'watermark_epoch');
            $currentComplete = $this->nullableInt($state, 'complete_through');
            if ($currentEpoch !== null
                && hash_equals($currentEpoch, $watermark->projectionEpoch)
                && $currentComplete !== null
                && $watermark->completeThrough <= $currentComplete) {
                return 'superseded';
            }

            DB::table('native_account_character_projection_state')->where('id', 1)->update([
                'watermark_epoch' => $watermark->projectionEpoch,
                'watermark_source_authority' => $watermark->sourceAuthority,
                'complete_through' => $watermark->completeThrough,
                'watermark_observed_at' => $watermark->observedAt,
                'updated_at' => now(),
            ]);

            return 'accepted';
        });
    }

    private function raiseEpoch(string $epoch): void
    {
        DB::table('native_account_character_snapshots')->update(['invalid' => true, 'updated_at' => now()]);
        DB::table('native_account_character_projection_state')->where('id', 1)->update([
            'highest_epoch' => $epoch,
            'watermark_epoch' => null,
            'watermark_source_authority' => null,
            'complete_through' => null,
            'watermark_observed_at' => null,
            'updated_at' => now(),
        ]);
    }

    private function string(object $row, string $column): string
    {
        $value = get_object_vars($row)[$column] ?? null;
        if (! is_string($value)) {
            throw new NativeAccountCharactersRefused(503);
        }

        return $value;
    }

    private function nullableString(object $row, string $column): ?string
    {
        $value = get_object_vars($row)[$column] ?? null;
        if ($value !== null && ! is_string($value)) {
            throw new NativeAccountCharactersRefused(503);
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
            throw new NativeAccountCharactersRefused(503);
        }

        return $value;
    }
}
