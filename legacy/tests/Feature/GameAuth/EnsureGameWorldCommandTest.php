<?php

namespace Tests\Feature\GameAuth;

use App\GameAuth\Worlds\GameWorld;
use App\GameAuth\Worlds\GameWorldStatus;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Artisan;
use Symfony\Component\Console\Command\Command;
use Tests\TestCase;

final class EnsureGameWorldCommandTest extends TestCase
{
    use RefreshDatabase;

    public function test_command_creates_and_idempotently_updates_an_explicit_world_route(): void
    {
        self::assertSame(Command::SUCCESS, Artisan::call('game-auth:world:ensure', [
            '--id' => '1',
            '--slug' => 'oteryn-staging',
            '--name' => 'Oteryn Staging',
            '--region' => 'LAN',
            '--host' => '192.168.1.2',
            '--port' => '7172',
            '--status' => 'online',
            '--login-enabled' => '1',
        ]));

        $world = GameWorld::query()->findOrFail(1);
        self::assertSame('oteryn-staging', $world->slug);
        self::assertSame(GameWorldStatus::Online, $world->status);
        self::assertTrue($world->login_enabled);
        self::assertSame('192.168.1.2', $world->game_host);
        self::assertSame(7172, $world->game_port);

        self::assertSame(Command::SUCCESS, Artisan::call('game-auth:world:ensure', [
            '--id' => '1',
            '--slug' => 'oteryn-staging',
            '--name' => 'Oteryn LAN',
            '--region' => 'HOME',
            '--host' => '192.168.1.2',
            '--port' => '7172',
            '--status' => 'maintenance',
            '--login-enabled' => '0',
        ]));

        self::assertSame(1, GameWorld::query()->count());
        $world->refresh();
        self::assertSame('Oteryn LAN', $world->name);
        self::assertSame('HOME', $world->region);
        self::assertSame(GameWorldStatus::Maintenance, $world->status);
        self::assertFalse($world->login_enabled);
    }

    public function test_command_rejects_invalid_or_unsafe_route_values(): void
    {
        $base = [
            '--id' => '1',
            '--slug' => 'oteryn-staging',
            '--name' => 'Oteryn Staging',
            '--region' => 'LAN',
            '--host' => '192.168.1.2',
            '--port' => '7172',
            '--status' => 'online',
            '--login-enabled' => '1',
        ];

        foreach ([
            ['--id' => '0'],
            ['--slug' => 'Invalid Slug'],
            ['--host' => 'http://192.168.1.2'],
            ['--port' => '0'],
            ['--port' => '65536'],
            ['--status' => 'starting'],
            ['--login-enabled' => 'maybe'],
        ] as $override) {
            self::assertSame(
                Command::FAILURE,
                Artisan::call('game-auth:world:ensure', array_replace($base, $override)),
            );
        }

        self::assertSame(0, GameWorld::query()->count());
    }

    public function test_command_fails_closed_when_slug_belongs_to_another_world(): void
    {
        GameWorld::query()->create([
            'slug' => 'oteryn-staging',
            'name' => 'Existing',
            'region' => 'LAN',
            'status' => GameWorldStatus::Online,
            'login_enabled' => true,
            'game_host' => '192.168.1.2',
            'game_port' => 7172,
        ]);

        self::assertSame(Command::FAILURE, Artisan::call('game-auth:world:ensure', [
            '--id' => '2',
            '--slug' => 'oteryn-staging',
            '--name' => 'Conflicting',
            '--region' => 'LAN',
            '--host' => '192.168.1.2',
            '--port' => '7172',
            '--status' => 'online',
            '--login-enabled' => '1',
        ]));

        self::assertSame(1, GameWorld::query()->count());
        self::assertNull(GameWorld::query()->find(2));
    }
}
