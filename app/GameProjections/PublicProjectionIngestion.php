<?php

namespace App\GameProjections;

use Illuminate\Support\Facades\DB;

/**
 * Applies snapshot and watermark publications (§5, §5.1, §6) to the read models.
 *
 * Every publication locks the singleton state row, so epoch raises, per-subject ordering and
 * watermark replacement are serialized. Per subject, a lower (epoch, revision) pair is
 * `superseded`, an equal pair with equal content is an idempotent `accepted` that keeps the first
 * stored `source_observed_at`, and an equal pair with different content is `409` with the first
 * stored snapshot kept. A higher epoch only raises the highest epoch seen; entries and watermarks
 * below it stop being served (PublicProjectionReadModel) until their new-epoch publication arrives.
 */
final class PublicProjectionIngestion
{
    private const CONFLICT = 'conflict';

    /** @return 'accepted'|'superseded' */
    public function snapshot(PublicProjectionSettings $settings, string $identity, PublicProjectionSnapshot $snapshot, int $now): string
    {
        if (! $settings->allows($identity, $snapshot->sourceAuthority)) {
            throw new PublicProjectionRefused(401);
        }
        if ($snapshot->sourceObservedAt > $now + PublicProjectionSettings::CLOCK_UNCERTAINTY_SECONDS) {
            throw new PublicProjectionRefused(400);
        }

        $result = DB::transaction(function () use ($snapshot): string {
            if (! $this->admitEpoch($snapshot->projectionEpoch)) {
                return 'superseded';
            }

            $query = fn () => DB::table('game_public_projection_snapshots')
                ->where('family', $snapshot->family->value)
                ->where('subject_key', $snapshot->subjectKey);
            $current = $query()->lockForUpdate()->first();
            if ($current !== null) {
                $order = PublicProjectionWire::compareUint64($snapshot->projectionEpoch, self::string($current, 'projection_epoch'))
                    ?: PublicProjectionWire::compareUint64($snapshot->projectionRevision, self::string($current, 'projection_revision'));
                if ($order < 0) {
                    return 'superseded';
                }
                if ($order === 0) {
                    return hash_equals(self::string($current, 'content_digest'), $snapshot->contentDigest)
                        ? 'accepted'
                        : self::CONFLICT;
                }
            }

            $values = [
                'world_id' => $snapshot->worldId,
                'lookup_name' => $snapshot->lookupName,
                'present' => $snapshot->present,
                'source_authority' => $snapshot->sourceAuthority,
                'projection_epoch' => $snapshot->projectionEpoch,
                'projection_revision' => $snapshot->projectionRevision,
                'source_observed_at' => $snapshot->sourceObservedAt,
                'content_digest' => $snapshot->contentDigest,
                'payload' => $snapshot->payload,
                'updated_at' => now(),
            ];
            if ($current === null) {
                DB::table('game_public_projection_snapshots')->insert($values + [
                    'family' => $snapshot->family->value,
                    'subject_key' => $snapshot->subjectKey,
                    'created_at' => now(),
                ]);
            } else {
                $query()->update($values);
            }

            return 'accepted';
        });

        if ($result === self::CONFLICT) {
            throw new PublicProjectionRefused(409);
        }

        return $result === 'accepted' ? 'accepted' : 'superseded';
    }

    /** @return 'accepted'|'superseded' */
    public function watermark(PublicProjectionSettings $settings, string $identity, PublicProjectionWatermark $watermark, int $now): string
    {
        if (! $settings->allows($identity, $watermark->sourceAuthority)) {
            throw new PublicProjectionRefused(401);
        }
        $latest = $now + PublicProjectionSettings::CLOCK_UNCERTAINTY_SECONDS;
        if ($watermark->observedAt > $latest || $watermark->completeThrough > $latest) {
            throw new PublicProjectionRefused(400);
        }

        return DB::transaction(function () use ($watermark): string {
            if (! $this->admitEpoch($watermark->projectionEpoch)) {
                return 'superseded';
            }

            $query = fn () => DB::table('game_public_projection_watermarks')->where('family', $watermark->family->value);
            $current = $query()->lockForUpdate()->first();
            if ($current !== null
                && hash_equals(self::string($current, 'projection_epoch'), $watermark->projectionEpoch)
                && $watermark->completeThrough <= self::int($current, 'complete_through')) {
                return 'superseded';
            }

            $values = [
                'source_authority' => $watermark->sourceAuthority,
                'projection_epoch' => $watermark->projectionEpoch,
                'complete_through' => $watermark->completeThrough,
                'observed_at' => $watermark->observedAt,
                'updated_at' => now(),
            ];
            if ($current === null) {
                DB::table('game_public_projection_watermarks')->insert($values + [
                    'family' => $watermark->family->value,
                    'created_at' => now(),
                ]);
            } else {
                $query()->update($values);
            }

            return 'accepted';
        });
    }

    /** Locks the state row; false when the epoch is below the highest seen, raising it when above. */
    private function admitEpoch(string $epoch): bool
    {
        $state = DB::table('game_public_projection_state')->where('id', 1)->lockForUpdate()->first();
        if ($state === null) {
            throw new PublicProjectionRefused(503);
        }

        $highest = self::nullableString($state, 'highest_epoch');
        $order = $highest === null ? 1 : PublicProjectionWire::compareUint64($epoch, $highest);
        if ($order > 0) {
            DB::table('game_public_projection_state')->where('id', 1)->update([
                'highest_epoch' => $epoch,
                'updated_at' => now(),
            ]);
        }

        return $order >= 0;
    }

    public static function string(object $row, string $column): string
    {
        $value = self::nullableString($row, $column);
        if ($value === null) {
            throw new PublicProjectionRefused(503);
        }

        return $value;
    }

    public static function nullableString(object $row, string $column): ?string
    {
        $value = get_object_vars($row)[$column] ?? null;
        if ($value !== null && ! is_string($value)) {
            throw new PublicProjectionRefused(503);
        }

        return $value;
    }

    public static function int(object $row, string $column): int
    {
        $value = get_object_vars($row)[$column] ?? null;
        if (is_string($value) && ctype_digit($value)) {
            $value = (int) $value;
        }
        if (! is_int($value)) {
            throw new PublicProjectionRefused(503);
        }

        return $value;
    }
}
