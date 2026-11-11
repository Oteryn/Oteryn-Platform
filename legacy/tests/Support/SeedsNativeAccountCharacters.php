<?php

namespace Tests\Support;

use Illuminate\Support\Facades\DB;

/** Seeds a live native account-character projection (the Platform side of ListCharactersForAccount). */
trait SeedsNativeAccountCharacters
{
    private function enableNativeAccountCharacters(): void
    {
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => ['CN=account-characters'],
            'game-auth.native_account_characters.source_authority' => 'oteryn:game:primary',
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
    }

    /** Marks the projection feed live as of now. */
    private function nativeAccountCharactersLive(): void
    {
        $now = now()->getTimestamp();
        DB::table('native_account_character_projection_state')->where('id', 1)->update([
            'highest_epoch' => '1',
            'watermark_epoch' => '1',
            'watermark_source_authority' => 'oteryn:game:primary',
            'complete_through' => $now - 1,
            'watermark_observed_at' => $now,
        ]);
    }

    /** @param  list<array{string, string, string}>  $characters character id, world id, name */
    private function bindAccountCharacters(string $accountId, array $characters): void
    {
        DB::table('native_account_character_snapshots')->updateOrInsert(['account_id' => $accountId], [
            'source_authority' => 'oteryn:game:primary',
            'projection_epoch' => '1',
            'projection_revision' => '1',
            'source_observed_at' => now()->getTimestamp(),
            'content_digest' => hash('sha256', $accountId),
            'invalid' => false,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
        foreach ($characters as [$characterId, $worldId, $name]) {
            DB::table('native_account_character_rows')->updateOrInsert(
                ['account_id' => $accountId, 'character_id' => $characterId],
                ['world_id' => $worldId, 'name' => $name, 'availability' => 'AVAILABLE', 'created_at' => now(), 'updated_at' => now()],
            );
        }
    }
}
