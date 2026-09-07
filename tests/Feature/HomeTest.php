<?php

namespace Tests\Feature;

use App\Cms\Models\NewsPost;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Redis\Connections\Connection;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Redis;
use Illuminate\Support\Facades\Schema;
use Mockery;
use Mockery\CompositeExpectation;
use Mockery\MockInterface;
use RuntimeException;
use Tests\TestCase;
use Throwable;

final class HomeTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        config()->set('database.connections.canary', [
            'driver' => 'sqlite',
            'database' => ':memory:',
            'prefix' => '',
            'foreign_key_constraints' => true,
        ]);

        DB::purge('canary');
        DB::connection('canary')->getPdo();

        Schema::connection('canary')->create('channels', function (Blueprint $table): void {
            $table->unsignedInteger('id')->primary();
            $table->string('name');
            $table->string('pvp_type');
            $table->unsignedInteger('max_players');
            $table->boolean('enabled');
            $table->integer('sort_order');
            $table->boolean('maintenance');
            $table->string('maintenance_message')->nullable();
        });
    }

    protected function tearDown(): void
    {
        DB::purge('canary');

        parent::tearDown();
    }

    public function test_homepage_renders_available_world_summary_and_latest_published_news(): void
    {
        $this->insertChannel();
        NewsPost::query()->create([
            'slug' => 'realm-awakens',
            'title' => 'The realm awakens',
            'body' => 'A published chronicle from the existing CMS boundary.',
            'published_at' => now()->subMinute(),
        ]);
        NewsPost::query()->create([
            'slug' => 'future-news',
            'title' => 'Future hidden news',
            'body' => 'This must remain hidden.',
            'published_at' => now()->addDay(),
        ]);
        $this->mockRuntime('ONLINE', 42, 25_000);

        $response = $this->get('/');

        $response->assertOk()
            ->assertSee('More than a game. A living world.')
            ->assertSee('Find your character')
            ->assertSeeText('42 players online')
            ->assertSee('The realm awakens')
            ->assertDontSee('Future hidden news')
            ->assertSee('data-content-state="AVAILABLE"', false)
            ->assertDontSee('css/home-preview.css', false)
            ->assertDontSee('css/app.css', false)
            ->assertSee('css/portal-system.css', false)
            ->assertSee('css/portal-art-direction.css', false)
            ->assertSee('<h1 id="home-hero-title" aria-label="Oteryn Platform">OTERYN</h1>', false)
            ->assertSee('css/home-production.css', false);
    }

    public function test_homepage_distinguishes_empty_world_and_news_states(): void
    {
        $this->get('/')
            ->assertOk()
            ->assertSee('No enabled worlds are configured.')
            ->assertSee('No news has been published yet.')
            ->assertSee('data-content-state="EMPTY"', false)
            ->assertDontSee('0 players online');
    }

    public function test_homepage_distinguishes_stale_runtime_without_synthetic_total(): void
    {
        $this->insertChannel();
        $this->mockRuntime('ONLINE', 25, -2);

        $this->get('/')
            ->assertOk()
            ->assertSee('runtime records are stale or missing')
            ->assertSee('data-content-state="STALE"', false)
            ->assertSee('Stale')
            ->assertDontSee('25 players online')
            ->assertDontSee('0 players online');
    }

    public function test_homepage_distinguishes_unavailable_runtime_without_reporting_offline_or_zero(): void
    {
        $this->insertChannel();

        $connection = Mockery::mock(Connection::class);
        Redis::shouldReceive('connection')->once()->with('canary_runtime')->andReturn($connection);
        $this->expectCommandThrows(
            $connection,
            'hmget',
            ['cluster:channel:1:runtime', ['channel_id', 'status', 'players_online']],
            new RuntimeException('Redis unavailable.'),
        );

        $this->get('/')
            ->assertOk()
            ->assertSee('Live world data is temporarily unavailable.')
            ->assertSee('data-content-state="UNAVAILABLE"', false)
            ->assertSee('Alpha')
            ->assertDontSee('0 players online')
            ->assertDontSee('OFFLINE');
    }

    public function test_homepage_keeps_guest_and_authenticated_account_states_distinct(): void
    {
        $this->get('/')
            ->assertOk()
            ->assertSee('Create account')
            ->assertSee('Sign in');

        $identity = Identity::query()->create([
            'email' => 'home-account@example.test',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $identity->refresh();

        $this->actingAs($identity)
            ->withSession([WebSessionState::GENERATION_KEY => $identity->web_session_generation])
            ->get('/')
            ->assertOk()
            ->assertSee('Open account center')
            ->assertSee('Sign out')
            ->assertDontSee('Recover password');
    }

    private function insertChannel(): void
    {
        DB::connection('canary')->table('channels')->insert([
            'id' => 1,
            'name' => 'Alpha',
            'pvp_type' => 'open-pvp',
            'max_players' => 500,
            'enabled' => 1,
            'sort_order' => 1,
            'maintenance' => 0,
            'maintenance_message' => null,
        ]);
    }

    private function mockRuntime(string $status, int $playersOnline, int $ttlMilliseconds): void
    {
        $connection = Mockery::mock(Connection::class);
        Redis::shouldReceive('connection')->once()->with('canary_runtime')->andReturn($connection);
        $this->expectCommandReturns(
            $connection,
            'hmget',
            ['cluster:channel:1:runtime', ['channel_id', 'status', 'players_online']],
            [
                'channel_id' => '1',
                'status' => $status,
                'players_online' => (string) $playersOnline,
            ],
        );
        $this->expectCommandReturns(
            $connection,
            'pttl',
            ['cluster:channel:1:runtime'],
            $ttlMilliseconds,
        );
    }

    /**
     * @param  list<mixed>  $arguments
     */
    private function expectCommandReturns(
        MockInterface $connection,
        string $command,
        array $arguments,
        mixed $result,
    ): void {
        $expectation = $this->commandExpectation($connection);
        $expectation->__call('once', []);
        $expectation->__call('with', [$command, $arguments]);
        $expectation->andReturn($result);
    }

    /**
     * @param  list<mixed>  $arguments
     */
    private function expectCommandThrows(
        MockInterface $connection,
        string $command,
        array $arguments,
        Throwable $exception,
    ): void {
        $expectation = $this->commandExpectation($connection);
        $expectation->__call('once', []);
        $expectation->__call('with', [$command, $arguments]);
        $expectation->__call('andThrow', [$exception]);
    }

    private function commandExpectation(MockInterface $connection): CompositeExpectation
    {
        /** @var CompositeExpectation */
        $expectation = $connection->shouldReceive('command');

        return $expectation;
    }
}
