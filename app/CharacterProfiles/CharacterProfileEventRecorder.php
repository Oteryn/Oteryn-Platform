<?php

namespace App\CharacterProfiles;

use Illuminate\Support\Facades\DB;

final class CharacterProfileEventRecorder
{
    public const PREFERENCES_UPDATED = 'character.profile_preferences_updated';

    public const MAIN_CHARACTER_SELECTED = 'character.main_character_selected';

    public function recordPreferencesUpdated(int $identityId): void
    {
        $this->record($identityId, self::PREFERENCES_UPDATED);
    }

    public function recordMainCharacterSelected(int $identityId): void
    {
        $this->record($identityId, self::MAIN_CHARACTER_SELECTED);
    }

    private function record(int $identityId, string $eventType): void
    {
        DB::table('identity_security_events')->insert([
            'identity_id' => $identityId,
            'event_type' => $eventType,
            'occurred_at' => now(),
        ]);
    }
}
