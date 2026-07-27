<?php

namespace Tests\Feature\PublicGameData;

use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

final class CharacterProfilePresentationTest extends TestCase
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
            $table->unsignedInteger('account_id');
            $table->integer('level');
            $table->integer('vocation');
            $table->bigInteger('deletion')->default(0);
        });
        Schema::connection('canary')->create('guilds', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
        });
        Schema::connection('canary')->create('guild_membership', function (Blueprint $table): void {
            $table->integer('player_id')->primary();
            $table->integer('guild_id');
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
            'deletion' => 0,
        ]);
        DB::connection('canary')->table('guilds')->insert([
            'id' => 7,
            'name' => 'Knights of Oteryn',
        ]);
        DB::connection('canary')->table('guild_membership')->insert([
            'player_id' => 1,
            'guild_id' => 7,
        ]);
        DB::connection('canary')->statement('PRAGMA query_only = ON');

        $this->get(route('game.characters.show', ['name' => 'Active Knight']))
            ->assertOk()
            ->assertSee('Character profile')
            ->assertSee('Active Knight')
            ->assertSee('120')
            ->assertSee('Knight')
            ->assertSee('Knights of Oteryn')
            ->assertSee(route('game.guilds.show', ['name' => 'Knights of Oteryn']), false)
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
            ->assertDontSee('515151');
    }
}
