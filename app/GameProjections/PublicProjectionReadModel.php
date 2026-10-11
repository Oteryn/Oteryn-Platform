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
        if (PublicProjectionSettings::current() === null) {
            return false;
        }

        $highest = $this->highestEpoch();
        $watermark = DB::table('game_public_projection_watermarks')->where('family', $family->value)->first();
        if ($highest === null || $watermark === null
            || ! hash_equals($highest, PublicProjectionIngestion::string($watermark, 'projection_epoch'))) {
            return false;
        }

        $completeThrough = PublicProjectionIngestion::int($watermark, 'complete_through');
        $uncertainty = PublicProjectionSettings::CLOCK_UNCERTAINTY_SECONDS;

        return $completeThrough <= $now + $uncertainty
            && $now - $completeThrough + $uncertainty <= PublicProjectionSettings::STALE_SECONDS;
    }

    /**
     * The stored content (subject members and body, object members sorted) of one present entry,
     * or null when the family is stale or the entry is absent, hidden or below the highest epoch.
     *
     * @return array<string, mixed>|null
     */
    public function entry(PublicProjectionFamily $family, string $subjectKey, int $now): ?array
    {
        if (! $this->fresh($family, $now)) {
            return null;
        }

        $row = DB::table('game_public_projection_snapshots')
            ->where('family', $family->value)
            ->where('subject_key', $subjectKey)
            ->first();
        if ($row === null
            || ! in_array(get_object_vars($row)['present'] ?? null, [true, 1, '1'], true)
            || $this->highestEpoch() !== PublicProjectionIngestion::string($row, 'projection_epoch')) {
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

    private function highestEpoch(): ?string
    {
        $state = DB::table('game_public_projection_state')->where('id', 1)->first();

        return $state === null ? null : PublicProjectionIngestion::nullableString($state, 'highest_epoch');
    }
}
