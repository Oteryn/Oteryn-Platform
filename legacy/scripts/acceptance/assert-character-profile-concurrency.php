<?php

declare(strict_types=1);

use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$identity = Identity::query()
    ->where('email', 'community-acceptance@example.test')
    ->first();
if (! $identity instanceof Identity) {
    fwrite(STDERR, "Acceptance identity is missing.\n");
    exit(2);
}

$preferences = CharacterProfilePreference::query()
    ->where('identity_id', $identity->id)
    ->orderBy('character_id')
    ->get();
$main = $preferences->where('is_main_character', true)->values();

if ($preferences->count() !== 2 || $main->count() !== 1) {
    fwrite(STDERR, json_encode([
        'result' => 'failure',
        'preference_count' => $preferences->count(),
        'main_count' => $main->count(),
    ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
    exit(3);
}

$mainCharacterId = $main->first()?->character_id;
if (! is_string($mainCharacterId) || $mainCharacterId === '') {
    fwrite(STDERR, "Main-character preference is not keyed by a native CharacterId.\n");
    exit(4);
}

fwrite(STDOUT, json_encode([
    'result' => 'pass',
    'preference_count' => 2,
    'main_count' => 1,
    'main_character_id' => $mainCharacterId,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
