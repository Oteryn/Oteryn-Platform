<?php

namespace Tests\Feature\PublicGameData;

use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

class GuildIndexTest extends TestCase
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

        Schema::connection('canary')->create('players', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->unsignedInteger('account_id')->default(0);
            $table->bigInteger('deletion')->default(0);
        });

        Schema::connection('canary')->create('guilds', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->integer('ownerid');
            $table->integer('level')->default(1);
            $table->bigInteger('creationdata')->default(0);
            $table->text('motd')->default('');
            $table->integer('residence')->default(0);
            $table->bigInteger('balance')->default(0);
            $table->integer('points')->default(0);
        });

        Schema::connection('canary')->create('guild_membership', function (Blueprint $table): void {
            $table->integer('player_id')->primary();
            $table->integer('guild_id');
            $table->integer('rank_id');
            $table->string('nick')->nullable();
        });
    }

    public function test_guild_index_is_alphabetical_paginated_private_field_safe_and_query_bounded(): void
    {
        DB::connection('canary')->table('players')->insert([
            [
                'id' => 1,
                'name' => 'Active Member',
                'account_id' => 42424242,
                'deletion' => 0,
            ],
            [
                'id' => 2,
                'name' => 'Deleted Member',
                'account_id' => 52525252,
                'deletion' => 1,
            ],
        ]);

        $guilds = [];
        for ($id = 51; $id >= 1; $id--) {
            $guilds[] = [
                'id' => $id,
                'name' => sprintf('Guild %02d', $id),
                'ownerid' => 900_000 + $id,
                'level' => 777,
                'creationdata' => 888_888_888,
                'motd' => $id === 1 ? 'PRIVATE-GUILD-MOTD' : '',
                'residence' => 999,
                'balance' => 987_654_321,
                'points' => 654_321,
            ];
        }

        DB::connection('canary')->table('guilds')->insert($guilds);
        DB::connection('canary')->table('guild_membership')->insert([
            ['player_id' => 1, 'guild_id' => 1, 'rank_id' => 10, 'nick' => 'Visible only on detail'],
            ['player_id' => 2, 'guild_id' => 1, 'rank_id' => 10, 'nick' => 'Deleted private nickname'],
        ]);

        $this->makeCanaryConnectionReadOnly();
        $connection = DB::connection('canary');
        $connection->flushQueryLog();
        $connection->enableQueryLog();

        $this->get(route('game.guilds.index'))
            ->assertOk()
            ->assertSeeInOrder([
                'Guild 01',
                '<td>1</td>',
                'Guild 02',
                '<td>0</td>',
                'Guild 50',
            ], false)
            ->assertDontSee('Guild 51')
            ->assertDontSee('PRIVATE-GUILD-MOTD')
            ->assertDontSee('987654321')
            ->assertDontSee('42424242')
            ->assertDontSee('Deleted Member');

        $this->assertLessThanOrEqual(2, count($connection->getQueryLog()));

        $this->get(route('game.guilds.index', ['page' => 2]))
            ->assertOk()
            ->assertSee('Guild 51');
    }

    public function test_guild_index_renders_a_successful_empty_state(): void
    {
        $this->makeCanaryConnectionReadOnly();

        $this->get(route('game.guilds.index'))
            ->assertOk()
            ->assertSee('No guilds found.');
    }

    public function test_guild_index_returns_service_unavailable_when_canary_query_fails(): void
    {
        Schema::connection('canary')->drop('guild_membership');
        $this->makeCanaryConnectionReadOnly();

        $this->get(route('game.guilds.index'))
            ->assertStatus(503)
            ->assertDontSee('No guilds found.');
    }

    private function makeCanaryConnectionReadOnly(): void
    {
        DB::connection('canary')->statement('PRAGMA query_only = ON');
    }
}
