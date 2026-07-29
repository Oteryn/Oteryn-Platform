<?php

declare(strict_types=1);

use App\CharacterProfiles\CharacterProfilePreferenceService;
use App\Identity\Models\Identity;
use Illuminate\Contracts\Console\Kernel;

require dirname(__DIR__, 2).'/vendor/autoload.php';

$app = require dirname(__DIR__, 2).'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$name = $argv[1] ?? null;
if (! is_string($name) || trim($name) === '') {
    fwrite(STDERR, "A character name argument is required.\n");
    exit(2);
}

$identity = Identity::query()
    ->where('email', 'community-acceptance@example.test')
    ->first();
if (! $identity instanceof Identity) {
    fwrite(STDERR, "Acceptance identity is missing.\n");
    exit(3);
}

$service = $app->make(CharacterProfilePreferenceService::class);
$service->update($identity, $name, [
    'public_comment' => 'Concurrent acceptance selection for '.$name,
    'show_account_association' => true,
    'show_status' => true,
    'show_guild' => true,
    'show_house' => true,
    'show_skills' => true,
    'show_deaths' => true,
    'show_kills' => true,
    'is_main_character' => true,
]);

fwrite(STDOUT, json_encode([
    'character' => $name,
    'result' => 'updated',
], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL);
