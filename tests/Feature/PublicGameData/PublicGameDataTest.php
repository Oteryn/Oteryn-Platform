<?php

namespace Tests\Feature\PublicGameData;

use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

class PublicGameDataTest extends TestCase
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

        Schema::connection('canary')->create('players', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->unsignedInteger('account_id')->default(0);
            $table->integer('group_id')->default(1);
            $table->integer('level')->default(1);
            $table->unsignedBigInteger('experience')->default(0);
            $table->integer('vocation')->default(0);
            $table->integer('maglevel')->default(0);
            $table->bigInteger('lastlogin')->default(0);
            $table->bigInteger('lastlogout')->default(0);
            $table->integer('boss_points')->default(0);
            $table->string('comment')->default('');
            $table->integer('skill_fist')->default(10);
            $table->integer('skill_club')->default(10);
            $table->integer('skill_sword')->default(10);
            $table->integer('skill_axe')->default(10);
            $table->integer('skill_dist')->default(10);
            $table->integer('skill_shielding')->default(10);
            $table->integer('skill_fishing')->default(10);
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

        Schema::connection('canary')->create('guild_ranks', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->integer('guild_id');
            $table->string('name');
            $table->integer('level');
        });

        Schema::connection('canary')->create('guild_membership', function (Blueprint $table): void {
            $table->integer('player_id')->primary();
            $table->integer('guild_id');
            $table->integer('rank_id');
            $table->string('nick')->nullable();
        });

        Schema::connection('canary')->create('houses', function (Blueprint $table): void {
            $table->integer('id');
            $table->integer('channel_id')->default(1);
            $table->integer('owner')->default(0);
            $table->string('name');
            $table->integer('size')->default(0);
            $table->primary(['channel_id', 'id']);
        });

        Schema::connection('canary')->create('player_deaths', function (Blueprint $table): void {
            $table->integer('player_id');
            $table->bigInteger('time');
            $table->integer('level');
            $table->string('killed_by');
            $table->boolean('is_player')->default(false);
        });

        Schema::connection('canary')->create('channels', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->string('pvp_type');
            $table->integer('max_players')->default(0);
            $table->boolean('enabled')->default(true);
            $table->integer('sort_order')->default(0);
            $table->boolean('maintenance')->default(false);
            $table->text('maintenance_message')->nullable();
        });

        Schema::connection('canary')->create('cluster_sessions', function (Blueprint $table): void {
            $table->integer('player_id')->primary();
            $table->unsignedInteger('account_id');
            $table->integer('channel_id');
            $table->string('instance_id');
            $table->string('session_id');
            $table->unsignedBigInteger('fencing_token');
            $table->string('status');
            $table->bigInteger('last_heartbeat');
            $table->bigInteger('expires_at');
        });
    }

    public function test_character_profile_exposes_only_approved_fields_and_hides_deleted_characters(): void
    {
        DB::connection('canary')->table('players')->insert([
            [
                'id' => 1,
                'name' => 'Active Knight',
                'account_id' => 424242,
                'level' => 120,
                'vocation' => 4,
                'deletion' => 0,
            ],
            [
                'id' => 2,
                'name' => 'Deleted Knight',
                'account_id' => 525252,
                'level' => 130,
                'vocation' => 4,
                'deletion' => 1,
            ],
        ]);
        $this->makeCanaryConnectionReadOnly();

        $this->get(route('game.characters.show', ['name' => 'Active Knight']))
            ->assertOk()
            ->assertSee('Active Knight')
            ->assertSee('120')
            ->assertSee('4')
            ->assertDontSee('424242');

        $this->get(route('game.characters.show', ['name' => 'Deleted Knight']))
            ->assertNotFound();
    }

    public function test_guild_read_model_escapes_content_excludes_deleted_members_and_avoids_n_plus_one_queries(): void
    {
        DB::connection('canary')->table('players')->insert([
            ['id' => 1, 'name' => 'Guild Leader', 'account_id' => 10, 'level' => 150, 'vocation' => 4, 'deletion' => 0],
            ['id' => 2, 'name' => 'Deleted Member', 'account_id' => 11, 'level' => 80, 'vocation' => 2, 'deletion' => 1],
        ]);
        DB::connection('canary')->table('guilds')->insert([
            'id' => 7,
            'name' => 'Knights',
            'ownerid' => 1,
            'level' => 3,
            'creationdata' => 123456,
            'motd' => '<script>alert(1)</script>',
            'residence' => 1,
            'balance' => 987654321,
            'points' => 42,
        ]);
        DB::connection('canary')->table('guild_ranks')->insert([
            ['id' => 1, 'guild_id' => 7, 'name' => 'The Leader', 'level' => 3],
            ['id' => 2, 'guild_id' => 7, 'name' => 'Member', 'level' => 1],
        ]);
        DB::connection('canary')->table('guild_membership')->insert([
            ['player_id' => 1, 'guild_id' => 7, 'rank_id' => 1, 'nick' => 'Boss'],
            ['player_id' => 2, 'guild_id' => 7, 'rank_id' => 2, 'nick' => null],
        ]);

        $this->makeCanaryConnectionReadOnly();
        $connection = DB::connection('canary');
        $connection->flushQueryLog();
        $connection->enableQueryLog();

        $this->get(route('game.guilds.show', ['name' => 'Knights']))
            ->assertOk()
            ->assertSee('Knights')
            ->assertSee('Guild Leader')
            ->assertDontSee('Deleted Member')
            ->assertDontSee('987654321')
            ->assertDontSee('<script>alert(1)</script>', false)
            ->assertSee('&lt;script&gt;alert(1)&lt;/script&gt;', false);

        $this->assertLessThanOrEqual(3, count($connection->getQueryLog()));
    }

    public function test_server_page_exposes_only_enabled_configured_channel_metadata_without_claiming_live_online_state(): void
    {
        DB::connection('canary')->table('channels')->insert([
            [
                'id' => 1,
                'name' => 'Main',
                'pvp_type' => 'pvp',
                'max_players' => 1000,
                'enabled' => 1,
                'sort_order' => 1,
                'maintenance' => 0,
                'maintenance_message' => null,
            ],
            [
                'id' => 2,
                'name' => 'Maintenance World',
                'pvp_type' => 'no-pvp',
                'max_players' => 500,
                'enabled' => 1,
                'sort_order' => 2,
                'maintenance' => 1,
                'maintenance_message' => 'Scheduled maintenance',
            ],
            [
                'id' => 3,
                'name' => 'Hidden Channel',
                'pvp_type' => 'pvp',
                'max_players' => 100,
                'enabled' => 0,
                'sort_order' => 3,
                'maintenance' => 0,
                'maintenance_message' => null,
            ],
        ]);
        $this->makeCanaryConnectionReadOnly();

        $this->get(route('game.servers.index'))
            ->assertOk()
            ->assertSee('Main')
            ->assertSee('Maintenance World')
            ->assertSee('Scheduled maintenance')
            ->assertDontSee('Hidden Channel')
            ->assertSee('live player availability is intentionally not shown', false);
    }

    private function makeCanaryConnectionReadOnly(): void
    {
        DB::connection('canary')->statement('PRAGMA query_only = ON');
    }
}
