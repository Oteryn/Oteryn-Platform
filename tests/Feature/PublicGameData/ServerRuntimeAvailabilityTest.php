<?php

namespace Tests\Feature\PublicGameData;

use DOMDocument;
use DOMElement;
use DOMXPath;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Redis\Connections\Connection;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Redis;
use Illuminate\Support\Facades\Schema;
use Illuminate\Testing\TestResponse;
use Mockery;
use Mockery\CompositeExpectation;
use Mockery\MockInterface;
use RuntimeException;
use Tests\TestCase;

final class ServerRuntimeAvailabilityTest extends TestCase
{
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
            $table->string('external_host');
            $table->unsignedSmallInteger('game_port');
            $table->unsignedSmallInteger('status_port');
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

    public function test_server_page_renders_explicit_runtime_states_counts_and_full_status(): void
    {
        $this->insertChannel(1, 'Alpha', 100, 1);
        $this->insertChannel(2, 'Beta', 200, 2);

        $connection = Mockery::mock(Connection::class);
        Redis::shouldReceive('connection')->twice()->with('canary_runtime')->andReturn($connection);
        $this->commandExpectation($connection)->__call('andReturnUsing', [
            function (string $command, array $arguments): mixed {
                $key = $arguments[0] ?? null;

                if ($command === 'hmget' && $key === 'cluster:channel:1:runtime') {
                    return [
                        'channel_id' => '1',
                        'status' => 'ONLINE',
                        'players_online' => '100',
                    ];
                }

                if ($command === 'pttl' && $key === 'cluster:channel:1:runtime') {
                    return 25000;
                }

                if ($command === 'hmget' && $key === 'cluster:channel:2:runtime') {
                    return [
                        'channel_id' => '2',
                        'status' => 'MAINTENANCE',
                        'players_online' => '3',
                    ];
                }

                if ($command === 'pttl' && $key === 'cluster:channel:2:runtime') {
                    return 25000;
                }

                throw new RuntimeException('Unexpected Redis command in test.');
            },
        ]);

        $response = $this->get(route('game.servers.index'));

        $response
            ->assertOk()
            ->assertSee('Alpha')
            ->assertSee('Beta')
            ->assertSee('Full')
            ->assertDontSee('instance_id')
            ->assertDontSee('build_sha')
            ->assertDontSee('map_hash')
            ->assertDontSee('data_hash');

        $this->assertWorldMetrics($response, [
            'Alpha' => ['Runtime:' => 'ONLINE', 'Players online:' => '100', 'Configured max players:' => '100', 'PvP type:' => 'pvp'],
            'Beta' => ['Runtime:' => 'MAINTENANCE', 'Players online:' => '3', 'Configured max players:' => '200', 'PvP type:' => 'pvp'],
        ]);
    }

    public function test_missing_or_expired_runtime_key_is_rendered_as_unknown_without_synthetic_count(): void
    {
        $this->insertChannel(1, 'Alpha', 100, 1);

        $connection = Mockery::mock(Connection::class);
        Redis::shouldReceive('connection')->once()->with('canary_runtime')->andReturn($connection);
        $this->expectCommandReturns($connection, 'hmget', [
            'cluster:channel:1:runtime',
            ['channel_id', 'status', 'players_online'],
        ], []);
        $this->expectCommandReturns($connection, 'pttl', ['cluster:channel:1:runtime'], -2);

        $response = $this->get(route('game.servers.index'));

        $response
            ->assertOk()
            ->assertSee('Alpha')
            ->assertDontSee('OFFLINE');

        $this->assertWorldMetrics($response, [
            'Alpha' => ['Runtime:' => 'Unknown', 'Players online:' => '—', 'Configured max players:' => '100', 'PvP type:' => 'pvp'],
        ]);
    }

    public function test_runtime_transport_failure_keeps_static_channels_but_marks_runtime_unavailable(): void
    {
        $this->insertChannel(1, 'Alpha', 100, 1);

        $connection = Mockery::mock(Connection::class);
        Redis::shouldReceive('connection')->once()->with('canary_runtime')->andReturn($connection);
        $expectation = $this->commandExpectation($connection);
        $expectation->__call('once', []);
        $expectation->__call('with', ['hmget', [
            'cluster:channel:1:runtime',
            ['channel_id', 'status', 'players_online'],
        ]]);
        $expectation->__call('andThrow', [new RuntimeException('Redis transport unavailable.')]);

        $response = $this->get(route('game.servers.index'));

        $response
            ->assertOk()
            ->assertSee('Alpha')
            ->assertSee('live player availability is intentionally not shown')
            ->assertDontSee('OFFLINE');

        $this->assertWorldMetrics($response, [
            'Alpha' => ['Runtime:' => 'Unavailable', 'Players online:' => '—', 'Configured max players:' => '100', 'PvP type:' => 'pvp'],
        ]);
    }

    public function test_failure_after_a_valid_channel_discards_the_entire_runtime_snapshot(): void
    {
        $this->insertChannel(1, 'Alpha', 100, 1);
        $this->insertChannel(2, 'Beta', 100, 2);

        $connection = Mockery::mock(Connection::class);
        Redis::shouldReceive('connection')->twice()->with('canary_runtime')->andReturn($connection);
        $this->commandExpectation($connection)->__call('andReturnUsing', [
            function (string $command, array $arguments): mixed {
                $key = $arguments[0] ?? null;

                if ($command === 'hmget' && $key === 'cluster:channel:1:runtime') {
                    return [
                        'channel_id' => '1',
                        'status' => 'ONLINE',
                        'players_online' => '25',
                    ];
                }

                if ($command === 'pttl' && $key === 'cluster:channel:1:runtime') {
                    return 25000;
                }

                if ($command === 'hmget' && $key === 'cluster:channel:2:runtime') {
                    throw new RuntimeException('Redis transport unavailable.');
                }

                throw new RuntimeException('Unexpected Redis command in test.');
            },
        ]);

        $response = $this->get(route('game.servers.index'));

        $response
            ->assertOk()
            ->assertSee('Alpha')
            ->assertSee('Beta')
            ->assertDontSee('data-world-state="online"', false);

        $this->assertWorldMetrics($response, [
            'Alpha' => ['Runtime:' => 'Unavailable', 'Players online:' => '—', 'Configured max players:' => '100', 'PvP type:' => 'pvp'],
            'Beta' => ['Runtime:' => 'Unavailable', 'Players online:' => '—', 'Configured max players:' => '100', 'PvP type:' => 'pvp'],
        ]);
    }

    /**
     * Match each world's labelled metrics, not the old inline <strong> markup.
     * An absent runtime must remain an em dash, never a fabricated zero/count.
     *
     * @param  array<string, array<string, string>>  $expected
     */
    private function assertWorldMetrics(TestResponse $response, array $expected): void
    {
        $document = new DOMDocument;
        $previous = libxml_use_internal_errors(true);
        try {
            $content = $response->getContent();
            self::assertIsString($content);
            self::assertTrue($document->loadHTML('<?xml encoding="utf-8" ?>'.$content));
        } finally {
            libxml_clear_errors();
            libxml_use_internal_errors($previous);
        }

        $xpath = new DOMXPath($document);
        $rows = $xpath->query('//article[contains(concat(" ", normalize-space(@class), " "), " world-row ")]');
        self::assertNotFalse($rows);
        $actual = [];
        foreach ($rows as $row) {
            self::assertInstanceOf(DOMElement::class, $row);
            $heading = $row->getElementsByTagName('h2')->item(0);
            self::assertInstanceOf(DOMElement::class, $heading);
            $name = trim($heading->textContent);
            $labels = $xpath->query('.//dl/div/dt', $row);
            self::assertNotFalse($labels);
            $metrics = [];
            foreach ($labels as $label) {
                self::assertInstanceOf(DOMElement::class, $label);
                $value = $label->nextElementSibling;
                self::assertInstanceOf(DOMElement::class, $value);
                self::assertSame('dd', $value->tagName);
                $metrics[trim($label->textContent)] = trim($value->textContent);
            }
            $actual[$name] = $metrics;
        }
        self::assertSame($expected, $actual);
    }

    /**
     * @param  list<mixed>  $arguments
     */
    private function expectCommandReturns(MockInterface $connection, string $command, array $arguments, mixed $returnValue): void
    {
        $expectation = $this->commandExpectation($connection);
        $expectation->__call('once', []);
        $expectation->__call('with', [$command, $arguments]);
        $expectation->andReturn($returnValue);
    }

    private function commandExpectation(MockInterface $connection): CompositeExpectation
    {
        /** @var CompositeExpectation */
        $expectation = $connection->shouldReceive('command');

        return $expectation;
    }

    private function insertChannel(int $id, string $name, int $maxPlayers, int $sortOrder): void
    {
        DB::connection('canary')->table('channels')->insert([
            'id' => $id,
            'name' => $name,
            'pvp_type' => 'pvp',
            'external_host' => strtolower($name).'.example.test',
            'game_port' => 7172 + $id,
            'status_port' => 7170 + $id,
            'max_players' => $maxPlayers,
            'enabled' => 1,
            'sort_order' => $sortOrder,
            'maintenance' => 0,
            'maintenance_message' => null,
        ]);
    }
}
