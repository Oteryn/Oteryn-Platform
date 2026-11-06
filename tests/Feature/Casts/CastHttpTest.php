<?php

namespace Tests\Feature\Casts;

use App\Casts\CastControls;
use App\GameAuth\NativeLogin\NativeGameLoginTickets;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use Tests\TestCase;

final class CastHttpTest extends TestCase
{
    use RefreshDatabase;

    public function test_node_api_is_default_off_and_rejects_unverified_peers(): void
    {
        $this->postJson('/internal/v1/casts/node', ['operation' => 'consume', 'ticket' => 'ocv1.'.str_repeat('a', 64)])
            ->assertStatus(503)->assertHeader('Cache-Control', 'no-store, private');
        config(['casts.enabled' => true, 'casts.nodes' => ['CN=cast-node' => '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b']]);
        $this->withHeaders(['SSL_CLIENT_VERIFY' => 'SUCCESS', 'SSL_PROTOCOL' => 'TLSv1.3', 'SSL_CLIENT_S_DN' => 'CN=cast-node'])
            ->postJson('/internal/v1/casts/node', ['operation' => 'release', 'reservation' => '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b'])
            ->assertStatus(401);
    }

    public function test_account_only_watch_ticket_is_consumed_once_by_the_verified_node(): void
    {
        $node = '0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b';
        $cast = str_repeat('a', 32);
        config(['casts.enabled' => true, 'casts.nodes' => ['CN=cast-node' => $node],
            'casts.routes' => [$node => ['host' => 'game.example.invalid', 'port' => 7172]],
            'game-auth.native_admission.enabled' => true,
            'game-auth.gateway.service_token_sha256' => hash('sha256', 'task-test-gateway')]);
        $owner = Identity::query()->create(['email' => 'http-owner@example.invalid', 'password' => 'unused']);
        $viewer = Identity::query()->create(['email' => 'http-viewer@example.invalid', 'password' => 'unused']);
        app(CastControls::class)->announce($node, $owner->account_id, $cast, 1, 1, 0, 50);
        $ticket = DB::transaction(fn () => app(NativeGameLoginTickets::class)->issue($viewer));
        $body = ['accountTicket' => $ticket->ticket, 'attemptRef' => (string) Str::uuid7(), 'cast' => $cast, 'sourceIp' => '127.0.0.1'];
        $watch = $this->withToken('task-test-gateway')->postJson('/internal/v1/casts/watch', $body);
        $watch->assertOk()->assertJsonPath('purpose', 'oteryn-cast-observer-v1')->assertJsonMissingPath('character_id');
        $this->withToken('task-test-gateway')->postJson('/internal/v1/casts/watch', $body)->assertStatus(409);
        $this->withServerVariables(['SSL_CLIENT_VERIFY' => 'SUCCESS', 'SSL_PROTOCOL' => 'TLSv1.3', 'SSL_CLIENT_S_DN' => 'CN=cast-node'])
            ->postJson('/internal/v1/casts/node', ['operation' => 'consume', 'ticket' => $watch->json('ticket')])
            ->assertOk()->assertJsonPath('account', $viewer->account_id)->assertJsonPath('node', $node)
            ->assertJsonPath('purpose', 'oteryn-cast-observer-v1')->assertJsonMissingPath('character_id');
        $this->postJson('/internal/v1/casts/node', ['operation' => 'consume', 'ticket' => $watch->json('ticket')])->assertStatus(409);
    }
}
