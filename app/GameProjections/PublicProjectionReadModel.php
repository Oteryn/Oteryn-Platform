<?php

namespace App\GameProjections;

use Illuminate\Support\Facades\DB;
use JsonException;

/**
 * Serve-time view of the public projection read models (§5.1, §6). Staleness is checked on every
 * read: a family whose watermark is missing, below the highest epoch, older than S or future-dated
 * serves nothing, and an entry below the highest epoch or published as absent is never served.
 * Page queries (removal plan packets C5-C7) build on this; the identifiers in the subject key are
 * internal join keys and must never reach a public page (§7.3).
 */
final class PublicProjectionReadModel
{
    public function fresh(PublicProjectionFamily $family, int $now): bool
    {
        return $this->freshEpoch($family, $now) !== null;
    }

    /**
     * The stored content (subject members and body, object members sorted) of one present entry,
     * or null when the family is stale or the entry is absent, hidden or below the highest epoch.
     *
     * @return array<string, mixed>|null
     */
    public function entry(PublicProjectionFamily $family, string $subjectKey, int $now): ?array
    {
        // The entry must carry the very epoch the watermark check validated, and that epoch must
        // still be the highest after the row is read (see freshEpoch()).
        $epoch = $this->freshEpoch($family, $now);
        if ($epoch === null) {
            return null;
        }

        $row = DB::table('game_public_projection_snapshots')
            ->where('family', $family->value)
            ->where('subject_key', $subjectKey)
            ->first();
        if ($row === null
            || ! in_array(get_object_vars($row)['present'] ?? null, [true, 1, '1'], true)
            || ! hash_equals($epoch, PublicProjectionIngestion::string($row, 'projection_epoch'))
            || $this->highestEpoch() !== $epoch) {
            return null;
        }

        try {
            $payload = json_decode(PublicProjectionIngestion::string($row, 'payload'), true, 8, JSON_THROW_ON_ERROR);
        } catch (JsonException) {
            return null;
        }
        if (! is_array($payload) || array_is_list($payload)) {
            return null;
        }

        /** @var array<string, mixed> $payload */
        return $payload;
    }

    /**
     * The distinct world ids that have a present entry of the family at the fresh epoch, or null
     * when the family is stale. Used to resolve the one served world; never shown on a page.
     *
     * @return list<string>|null
     */
    public function worlds(PublicProjectionFamily $family, int $now): ?array
    {
        $epoch = $this->freshEpoch($family, $now);
        if ($epoch === null) {
            return null;
        }

        $worlds = [];
        $rows = DB::table('game_public_projection_snapshots')
            ->where('family', $family->value)
            ->where('projection_epoch', $epoch)
            ->where('present', true)
            ->whereNotNull('world_id')
            ->distinct()
            ->orderBy('world_id')
            ->pluck('world_id');
        foreach ($rows as $world) {
            if (is_string($world) && $world !== '') {
                $worlds[] = $world;
            }
        }

        return $this->highestEpoch() === $epoch ? $worlds : null;
    }

    /**
     * The stored content of the present entries with the given subject keys, keyed by subject key,
     * or null when the family is stale. Keys without a servable entry are simply absent, so a
     * caller joining on them leaves the row out.
     *
     * @param  list<string>  $subjectKeys
     * @return array<string, array<string, mixed>>|null
     */
    public function entries(PublicProjectionFamily $family, array $subjectKeys, int $now): ?array
    {
        $epoch = $this->freshEpoch($family, $now);
        if ($epoch === null) {
            return null;
        }

        $found = [];
        foreach (array_chunk(array_values(array_unique($subjectKeys)), 500) as $chunk) {
            $rows = DB::table('game_public_projection_snapshots')
                ->where('family', $family->value)
                ->where('projection_epoch', $epoch)
                ->where('present', true)
                ->whereIn('subject_key', $chunk)
                ->get();
            foreach ($rows as $row) {
                try {
                    $payload = json_decode(PublicProjectionIngestion::string($row, 'payload'), true, 8, JSON_THROW_ON_ERROR);
                } catch (JsonException) {
                    continue;
                }
                if (is_array($payload) && ! array_is_list($payload)) {
                    /** @var array<string, mixed> $payload */
                    $found[PublicProjectionIngestion::string($row, 'subject_key')] = $payload;
                }
            }
        }

        return $this->highestEpoch() === $epoch ? $found : null;
    }

    /**
     * The highest epoch when the family's watermark is at that epoch and fresh, otherwise null.
     * The highest epoch only rises, so reading the same value before and after the watermark (and,
     * in entry(), after the row) proves every read in between saw that epoch as the highest.
     */
    private function freshEpoch(PublicProjectionFamily $family, int $now): ?string
    {
        if (PublicProjectionSettings::current() === null) {
            return null;
        }

        $highest = $this->highestEpoch();
        $watermark = DB::table('game_public_projection_watermarks')->where('family', $family->value)->first();
        if ($highest === null || $watermark === null
            || ! hash_equals($highest, PublicProjectionIngestion::string($watermark, 'projection_epoch'))) {
            return null;
        }

        $completeThrough = PublicProjectionIngestion::int($watermark, 'complete_through');
        $uncertainty = PublicProjectionSettings::CLOCK_UNCERTAINTY_SECONDS;
        if ($completeThrough > $now + $uncertainty
            || $now - $completeThrough + $uncertainty > PublicProjectionSettings::STALE_SECONDS) {
            return null;
        }

        return $this->highestEpoch() === $highest ? $highest : null;
    }

    private function highestEpoch(): ?string
    {
        $state = DB::table('game_public_projection_state')->where('id', 1)->first();

        return $state === null ? null : PublicProjectionIngestion::nullableString($state, 'highest_epoch');
    }
}
