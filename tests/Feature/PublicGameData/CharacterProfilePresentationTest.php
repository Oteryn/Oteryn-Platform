<?php

namespace Tests\Feature\PublicGameData;

use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

final class CharacterProfilePresentationTest extends TestCase
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
            $table->unsignedInteger('account_id');
            $table->integer('level');
            $table->integer('vocation');
            $table->bigInteger('experience')->default(0);
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
            $table->integer('ownerid')->default(0);
            $table->integer('level')->default(1);
            $table->bigInteger('creationdata')->default(0);
            $table->text('motd')->default('');
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
            $table->string('nick')->default('');
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
        Schema::connection('canary')->create('cluster_sessions', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->integer('player_id');
            $table->integer('channel_id');
            $table->string('status');
            $table->bigInteger('expires_at');
        });
    }

    protected function tearDown(): void
    {
        DB::purge('canary');

        parent::tearDown();
    }

    public function test_profile_renders_readable_vocation_and_linked_guild_without_account_data(): void
    {
        DB::connection('canary')->table('players')->insert([
            'id' => 1,
            'name' => 'Active Knight',
            'account_id' => 424242,
            'level' => 120,
            'vocation' => 4,
            'comment' => 'Public knight comment.',
            'deletion' => 0,
        ]);
        DB::connection('canary')->table('guilds')->insert([
            'id' => 7,
            'name' => 'Knights of Oteryn',
            'ownerid' => 1,
        ]);
        DB::connection('canary')->table('guild_ranks')->insert([
            'id' => 8,
            'guild_id' => 7,
            'name' => 'Leader',
            'level' => 3,
        ]);
        DB::connection('canary')->table('guild_membership')->insert([
            'player_id' => 1,
            'guild_id' => 7,
            'rank_id' => 8,
            'nick' => '',
        ]);
        DB::connection('canary')->statement('PRAGMA query_only = ON');

        $this->get(route('game.characters.show', ['name' => 'Active Knight']))
            ->assertOk()
            ->assertSee('Character profile')
            ->assertSee('Active Knight')
            ->assertSee('120')
            ->assertSee('Knight')
            ->assertSee('Knights of Oteryn')
            ->assertSee('Leader')
            ->assertSee('Public knight comment.')
            ->assertSee(route('game.guilds.show', ['name' => 'Knights of Oteryn']), false)
            ->assertSee('Status details are private.')
            ->assertSee('Account association is private.')
            ->assertDontSee('Vocation ID')
            ->assertDontSee('424242');
    }

    public function test_polish_profile_uses_explicit_no_guild_state_instead_of_an_empty_value(): void
    {
        DB::connection('canary')->table('players')->insert([
            'id' => 2,
            'name' => 'Solo Monk',
            'account_id' => 515151,
            'level' => 75,
            'vocation' => 9,
            'deletion' => 0,
        ]);
        DB::connection('canary')->statement('PRAGMA query_only = ON');

        $this->get('/pl/characters/Solo%20Monk')
            ->assertOk()
            ->assertSee('Profil postaci')
            ->assertSee('Solo Monk')
            ->assertSee('Monk')
            ->assertSee('Brak gildii')
            ->assertSee('Szczegóły statusu są prywatne.')
            ->assertDontSee('515151');
    }
}
