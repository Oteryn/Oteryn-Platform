<?php

namespace Tests\Feature\PublicGameData;

use App\PublicGameData\PublicGameDataSource;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Redis;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\TestCase;

final class PublicGameDataSourceTest extends TestCase
{
    use RefreshDatabase;

    /**
     * @return list<array{string, array<string, string>}>
     */
    public static function publicPages(): array
    {
        return [
            ['game.guilds.show', ['name' => 'Some Guild']],
            ['game.guilds.index', []],
            ['game.servers.index', []],
            ['game.deaths.index', []],
        ];
    }

    /**
     * @param  array<string, string>  $parameters
     */
    #[DataProvider('publicPages')]
    public function test_pages_render_unavailable_state_without_a_legacy_connection(string $route, array $parameters): void
    {
        config()->set('database.connections.'.PublicGameDataSource::CONNECTION, null);
        Redis::shouldReceive('connection')->never();

        $this->get(route($route, $parameters))
            ->assertOk()
            ->assertSee('community-unavailable', false)
            ->assertDontSee('SQLSTATE');
    }

    /**
     * @param  array<string, string>  $parameters
     */
    #[DataProvider('publicPages')]
    public function test_native_source_never_queries_a_configured_legacy_connection(string $route, array $parameters): void
    {
        $this->configureLegacyConnection();
        config()->set('public_game_data.source', PublicGameDataSource::NATIVE);
        Redis::shouldReceive('connection')->never();
        $connection = DB::connection(PublicGameDataSource::CONNECTION);
        $connection->enableQueryLog();

        $this->get(route($route, $parameters))
            ->assertOk()
            ->assertSee('community-unavailable', false);

        $this->assertSame([], $connection->getQueryLog());
    }

    /**
     * Mirrors the deployed shape: only the process environment variable is set and
     * no application config override exists.
     *
     * @param  array<string, string>  $parameters
     */
    #[DataProvider('publicPages')]
    public function test_process_environment_variable_alone_switches_pages_to_the_unavailable_state(string $route, array $parameters): void
    {
        $this->configureLegacyConnection();
        config()->set('public_game_data.source', null);
        Redis::shouldReceive('connection')->never();
        $connection = DB::connection(PublicGameDataSource::CONNECTION);
        $connection->enableQueryLog();
        putenv('PUBLIC_GAME_DATA_SOURCE=native');

        try {
            $this->get(route($route, $parameters))
                ->assertOk()
                ->assertSee('community-unavailable', false);
        } finally {
            putenv('PUBLIC_GAME_DATA_SOURCE');
        }

        $this->assertSame([], $connection->getQueryLog());
    }

    /**
     * Online, highscores and character pages are native-only: they never open the legacy connection, whatever
     * the source setting is, and answer 503 while no fresh projection exists.
     *
     * @param  array<string, string>  $parameters
     */
    #[DataProvider('nativeOnlyPages')]
    public function test_native_only_pages_never_open_the_legacy_connection(string $route, ?string $source, array $parameters = []): void
    {
        $this->configureLegacyConnection();
        config()->set('public_game_data.source', $source);
        Redis::shouldReceive('connection')->never();
        $connection = DB::connection(PublicGameDataSource::CONNECTION);
        $connection->enableQueryLog();

        $this->get(route($route, $parameters))
            ->assertStatus(503)
            ->assertSee('community-unavailable', false);

        $this->assertSame([], $connection->getQueryLog());
    }

    /** @return list<array{string, ?string}|array{string, ?string, array<string, string>}> */
    public static function nativeOnlyPages(): array
    {
        return [
            ['game.highscores.index', null],
            ['game.highscores.index', PublicGameDataSource::NATIVE],
            ['game.online.index', null],
            ['game.online.index', PublicGameDataSource::NATIVE],
            ['game.characters.show', null, ['name' => 'Some Hero']],
            ['game.characters.show', PublicGameDataSource::NATIVE, ['name' => 'Some Hero']],
        ];
    }

    public function test_homepage_is_ok_with_unavailable_world_state_when_source_is_native_or_absent(): void
    {
        config()->set('database.connections.'.PublicGameDataSource::CONNECTION, null);
        Redis::shouldReceive('connection')->never();

        $this->get('/')
            ->assertOk()
            ->assertSee('data-content-state="UNAVAILABLE"', false);

        $this->configureLegacyConnection();
        config()->set('public_game_data.source', PublicGameDataSource::NATIVE);
        $connection = DB::connection(PublicGameDataSource::CONNECTION);
        $connection->enableQueryLog();

        $this->get('/')
            ->assertOk()
            ->assertSee('data-content-state="UNAVAILABLE"', false);

        $this->assertSame([], $connection->getQueryLog());
    }

    public function test_source_resolution_defaults_to_the_configured_connection(): void
    {
        $this->configureLegacyConnection();

        $this->assertFalse(PublicGameDataSource::isNative());
        $this->assertTrue(PublicGameDataSource::isAvailable());

        config()->set('public_game_data.source', 'NATIVE');

        $this->assertTrue(PublicGameDataSource::isNative());
        $this->assertFalse(PublicGameDataSource::isAvailable());
    }

    private function configureLegacyConnection(): void
    {
        config()->set('database.connections.'.PublicGameDataSource::CONNECTION, [
            'driver' => 'sqlite',
            'database' => ':memory:',
            'prefix' => '',
            'foreign_key_constraints' => true,
        ]);
        DB::purge(PublicGameDataSource::CONNECTION);
    }
}
