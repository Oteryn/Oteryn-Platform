<?php

namespace Tests\Feature\PublicGameData;

use App\Accounts\Models\IdentityCanaryAccount;
use App\Identity\Models\Identity;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

class CommunityDataCompletenessTest extends TestCase
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

        $this->createCanarySchema();
        $this->seedCanaryCommunityData();
    }

    public function test_highscores_support_allowlisted_categories_vocation_filter_and_global_scope(): void
    {
        $this->get(route('game.highscores.index', [
            'category' => 'magic',
            'vocation' => 4,
            'scope' => 'global',
        ]))
            ->assertOk()
            ->assertSee('Highscores')
            ->assertSee('Acceptance Hero')
            ->assertSee('12')
            ->assertDontSee('Acceptance Mage')
            ->assertSee('Characters are global in the current Oteryn server model.');

        $this->from(route('game.highscores.index'))
            ->get(route('game.highscores.index', ['category' => 'unsupported']))
            ->assertRedirect(route('game.highscores.index'))
            ->assertSessionHasErrors('category');
    }

    public function test_character_profile_applies_private_defaults_then_public_account_and_status_flags(): void
    {
        $identity = $this->bindIdentity(publicAssociation: false, publicStatus: false);

        $this->get(route('game.characters.show', ['name' => 'Acceptance Hero']))
            ->assertOk()
            ->assertSee('A deterministic public hero comment.')
            ->assertSee('Acceptance Hall')
            ->assertSee('Acceptance Dragon')
            ->assertSee('Status details are private.')
            ->assertSee('Account association is private.')
            ->assertDontSee('Acceptance Mage');

        $identity->forceFill([
            'public_account_association' => true,
            'public_status_visible' => true,
        ])->save();

        $this->get(route('game.characters.show', ['name' => 'Acceptance Hero']))
            ->assertOk()
            ->assertSee('Online')
            ->assertSee('Acceptance Mage')
            ->assertDontSee('9001');
    }

    public function test_latest_deaths_guild_search_localization_and_read_only_policy_are_explicit(): void
    {
        $this->get(route('game.deaths.index'))
            ->assertOk()
            ->assertSee('Latest deaths')
            ->assertSee('Acceptance Hero')
            ->assertSee('Acceptance Dragon')
            ->assertSee('no authoritative world-transfer source');

        $this->get(route('game.guilds.index', ['q' => 'Acceptance']))
            ->assertOk()
            ->assertSee('Acceptance Guild')
            ->assertSee('Guild administration is not exposed by the Platform');

        $this->get(route('game.guilds.index', ['q' => 'No such guild']))
            ->assertOk()
            ->assertSee('No guilds match this search.');

        $this->get(route('game.guilds.show', ['name' => 'Acceptance Guild']))
            ->assertOk()
            ->assertSee('Acceptance Hero')
            ->assertDontSee('Owner player ID')
            ->assertDontSee('9001');

        $this->get(route('game.deaths.index', ['locale' => 'pl']))
            ->assertOk()
            ->assertSee('Ostatnie zgony');
    }

    public function test_dependency_failure_returns_localized_503_without_sql_details(): void
    {
        Schema::connection('canary')->drop('player_deaths');

        $this->get(route('game.deaths.index', ['locale' => 'pl']))
            ->assertStatus(503)
            ->assertSee('Dane społeczności są tymczasowo niedostępne')
            ->assertDontSee('SQLSTATE')
            ->assertDontSee('player_deaths');
    }

    public function test_complete_community_surfaces_remain_read_only(): void
    {
        $this->bindIdentity(publicAssociation: true, publicStatus: true);
        DB::connection('canary')->statement('PRAGMA query_only = ON');

        $this->get(route('game.highscores.index', ['category' => 'level']))->assertOk();
        $this->get(route('game.characters.show', ['name' => 'Acceptance Hero']))->assertOk();
        $this->get(route('game.deaths.index'))->assertOk();
        $this->get(route('game.guilds.index', ['q' => 'Acceptance']))->assertOk();
    }

    private function bindIdentity(bool $publicAssociation, bool $publicStatus): Identity
    {
        $identity = Identity::query()->create([
            'email' => 'community-profile@example.test',
            'password' => Hash::make('not-a-real-user-password'),
            'public_account_association' => $publicAssociation,
            'public_status_visible' => $publicStatus,
        ]);

        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => 9001,
            'provisioning_name' => 'community-profile',
            'canary_creation_epoch' => 1,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'last_failure_code' => null,
            'last_attempt_at' => now(),
            'ready_at' => now(),
        ]);

        return $identity;
    }

    private function createCanarySchema(): void
    {
        Schema::connection('canary')->create('players', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name')->unique();
            $table->integer('account_id');
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
            $table->integer('ownerid');
            $table->integer('level')->default(1);
            $table->bigInteger('creationdata')->default(0);
            $table->text('motd')->default('');
            $table->integer('residence')->default(0);
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
        Schema::connection('canary')->create('channels', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->string('name');
            $table->string('pvp_type')->default('open');
            $table->integer('max_players')->default(1000);
            $table->boolean('maintenance')->default(false);
            $table->string('maintenance_message')->nullable();
            $table->boolean('enabled')->default(true);
            $table->integer('sort_order')->default(0);
        });
        Schema::connection('canary')->create('cluster_sessions', function (Blueprint $table): void {
            $table->integer('id')->primary();
            $table->integer('player_id');
            $table->integer('channel_id');
            $table->string('status');
            $table->bigInteger('expires_at');
        });
    }

    private function seedCanaryCommunityData(): void
    {
        DB::connection('canary')->table('players')->insert([
            [
                'id' => 9001,
                'name' => 'Acceptance Hero',
                'account_id' => 9001,
                'level' => 42,
                'vocation' => 4,
                'experience' => 4_200_000,
                'maglevel' => 12,
                'lastlogin' => now()->subMinutes(5)->timestamp,
                'lastlogout' => now()->subMinutes(15)->timestamp,
                'boss_points' => 77,
                'comment' => 'A deterministic public hero comment.',
                'skill_fist' => 35,
                'skill_club' => 45,
                'skill_sword' => 80,
                'skill_axe' => 50,
                'skill_dist' => 60,
                'skill_shielding' => 75,
                'skill_fishing' => 20,
                'deletion' => 0,
            ],
            [
                'id' => 9002,
                'name' => 'Acceptance Mage',
                'account_id' => 9001,
                'level' => 40,
                'vocation' => 1,
                'experience' => 3_900_000,
                'maglevel' => 90,
                'lastlogin' => 0,
                'lastlogout' => 0,
                'boss_points' => 10,
                'comment' => '',
                'skill_fist' => 10,
                'skill_club' => 10,
                'skill_sword' => 10,
                'skill_axe' => 10,
                'skill_dist' => 15,
                'skill_shielding' => 30,
                'skill_fishing' => 10,
                'deletion' => 0,
            ],
            [
                'id' => 9003,
                'name' => 'Acceptance Victim',
                'account_id' => 9002,
                'level' => 30,
                'vocation' => 2,
                'experience' => 1_500_000,
                'maglevel' => 18,
                'lastlogin' => 0,
                'lastlogout' => 0,
                'boss_points' => 5,
                'comment' => '',
                'skill_fist' => 10,
                'skill_club' => 10,
                'skill_sword' => 10,
                'skill_axe' => 10,
                'skill_dist' => 20,
                'skill_shielding' => 25,
                'skill_fishing' => 10,
                'deletion' => 0,
            ],
        ]);
        DB::connection('canary')->table('guilds')->insert([
            'id' => 9001,
            'name' => 'Acceptance Guild',
            'ownerid' => 9001,
            'level' => 3,
            'creationdata' => 0,
            'motd' => 'Deterministic acceptance guild',
            'residence' => 1,
            'points' => 100,
        ]);
        DB::connection('canary')->table('guild_ranks')->insert([
            ['id' => 1, 'guild_id' => 9001, 'name' => 'Leader', 'level' => 3],
            ['id' => 2, 'guild_id' => 9001, 'name' => 'Member', 'level' => 1],
        ]);
        DB::connection('canary')->table('guild_membership')->insert([
            ['player_id' => 9001, 'guild_id' => 9001, 'rank_id' => 1, 'nick' => ''],
        ]);
        DB::connection('canary')->table('houses')->insert([
            'id' => 100,
            'channel_id' => 1,
            'owner' => 9001,
            'name' => 'Acceptance Hall',
            'size' => 42,
        ]);
        DB::connection('canary')->table('player_deaths')->insert([
            [
                'player_id' => 9001,
                'time' => now()->subMinute()->timestamp,
                'level' => 42,
                'killed_by' => 'Acceptance Dragon',
                'is_player' => false,
            ],
            [
                'player_id' => 9003,
                'time' => now()->subSeconds(30)->timestamp,
                'level' => 30,
                'killed_by' => 'Acceptance Hero',
                'is_player' => true,
            ],
        ]);
        DB::connection('canary')->table('channels')->insert([
            'id' => 1,
            'name' => 'Acceptance',
            'pvp_type' => 'open',
            'max_players' => 1000,
            'maintenance' => false,
            'maintenance_message' => null,
            'enabled' => true,
            'sort_order' => 1,
        ]);
        DB::connection('canary')->table('cluster_sessions')->insert([
            'id' => 1,
            'player_id' => 9001,
            'channel_id' => 1,
            'status' => 'ONLINE',
            'expires_at' => (int) floor(microtime(true) * 1000) + 3_600_000,
        ]);
    }
}
