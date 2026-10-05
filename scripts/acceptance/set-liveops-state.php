<?php

declare(strict_types=1);

use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\DB;

require __DIR__.'/../../vendor/autoload.php';

$app = require __DIR__.'/../../bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

if (env('APP_ENV') !== 'acceptance') {
    fwrite(STDERR, "Refusing to mutate LiveOps fixture outside acceptance.\n");
    exit(2);
}

$state = $argv[1] ?? '';
if (! in_array($state, ['ready', 'stale', 'invalid', 'unavailable', 'maintenance', 'recovery'], true)) {
    fwrite(STDERR, "Usage: php scripts/acceptance/set-liveops-state.php ready|stale|invalid|unavailable|maintenance|recovery\n");
    exit(2);
}

$worldId = '018f0f1e-7b2c-7a31-8d4e-1234567890ab';
$channelId = '018f0f1e-7b2c-7a32-8d4e-1234567890ac';
$nodeIdentity = 'CN=acceptance-runtime-node';
$now = now()->getTimestamp();

DB::transaction(function () use ($state, $worldId, $channelId, $nodeIdentity, $now): void {
    DB::table('game_worlds')->where('world_id', $worldId)->update([
        'status' => $state === 'maintenance' ? 'maintenance' : 'online',
        'login_enabled' => $state !== 'maintenance',
        'updated_at' => now(),
    ]);

    DB::table('native_scope_assignments')->updateOrInsert(
        ['world_id' => $worldId, 'channel_id' => $channelId],
        [
            'assignment_epoch' => '1',
            'ownership_generation' => '1',
            'node_identity' => $nodeIdentity,
            'assigned_at' => $now - 2,
            'created_at' => now(),
            'updated_at' => now(),
        ],
    );

    if ($state === 'unavailable') {
        DB::table('native_runtime_status_reports')
            ->where('world_id', $worldId)
            ->where('channel_id', $channelId)
            ->delete();

        return;
    }

    $observedAt = $state === 'stale' ? $now - 60 : $now - 2;
    DB::table('native_runtime_status_reports')->updateOrInsert(
        ['world_id' => $worldId, 'channel_id' => $channelId],
        [
            'node_identity' => $nodeIdentity,
            'source_authority' => 'oteryn-game',
            'node_id' => '11111111-2222-3333-8444-555555555555',
            'assignment_epoch' => '1',
            'scope_ownership_generation' => '1',
            'source_revision' => '1',
            'decision_identity' => 'acceptance-runtime',
            'ready' => true,
            'published_at' => $observedAt - 1,
            'observed_at' => $observedAt,
            'protocol_major' => 1,
            'transport_profile' => 1,
            'route_revision' => 'acceptance-route-1',
            'runtime_observation_revision' => 'acceptance-runtime-1',
            'ruleset_revision' => 'acceptance-ruleset-1',
            'content_revision' => 'acceptance-content-1',
            'map_revision' => 'acceptance-map-1',
            'world_policy_revision' => 'acceptance-policy-1',
            'offer_revision' => 'acceptance-offer-1',
            'content_digest' => str_repeat('b', 64),
            'invalid' => $state === 'invalid',
            'created_at' => now(),
            'updated_at' => now(),
        ],
    );
});

fwrite(STDOUT, "acceptance-liveops-state: {$state}\n");
