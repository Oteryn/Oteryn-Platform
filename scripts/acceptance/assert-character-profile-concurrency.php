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
    ->orderBy('canary_player_id')
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

$mainPlayerId = $main->first()?->canary_player_id;
if (! in_array($mainPlayerId, [9001, 9002], true)) {
    fwrite(STDERR, "Unexpected main-character player identifier.\n");
    exit(4);
}

fwrite(STDOUT, json_encode([
    'result' => 'pass',
    'preference_count' => 2,
    'main_count' => 1,
    'main_player_id' => $mainPlayerId,
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
