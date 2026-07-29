<?php

namespace Tests\Feature\CharacterProfiles;

use App\Accounts\Models\IdentityCanaryAccount;
use App\CharacterProfiles\CharacterProfileEventRecorder;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use Tests\TestCase;

final class CharacterProfilePreferenceTest extends TestCase
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

    public function test_guest_cannot_manage_character_profile_preferences(): void
    {
        $this->get('/account/characters/Alpha/profile')->assertRedirect('/login');
        $this->put('/account/characters/Alpha/profile')->assertRedirect('/login');
    }

    public function test_missing_preferences_preserve_existing_public_profile_behavior(): void
    {
        $identity = $this->identityWithBinding(7001, true, true);
        $this->seedCharacters();

        $this->get(route('game.characters.show', ['name' => 'Alpha Knight']))
            ->assertOk()
            ->assertSee('Canary comment')
            ->assertSee('Beta Druid')
            ->assertSee('Online')
            ->assertDontSee('Main character');

        self::assertSame(0, CharacterProfilePreference::query()->count());
        self::assertTrue($identity->public_account_association);
    }

    public function test_owner_can_update_escaped_comment_visibility_and_main_character(): void
    {
        $identity = $this->identityWithBinding(7001, true, true);
        $this->seedCharacters();
        $this->loginAsCurrentIdentity($identity);

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->assertOk()
            ->assertSee('Alpha Knight')
            ->assertSee('Public profile settings');

        $this->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), [
            'public_comment' => '<script>alert(1)</script> Owner text',
            'show_account_association' => '1',
            'show_status' => '1',
            'is_main_character' => '1',
        ])->assertRedirect(route('account.characters.profile.edit', ['name' => 'Alpha Knight']));

        $alpha = CharacterProfilePreference::query()->where('canary_player_id', 1)->firstOrFail();
        self::assertSame('<script>alert(1)</script> Owner text', $alpha->public_comment);
        self::assertTrue($alpha->is_main_character);
        self::assertFalse($alpha->show_guild);
        self::assertFalse($alpha->show_house);
        self::assertFalse($alpha->show_skills);
        self::assertFalse($alpha->show_deaths);
        self::assertFalse($alpha->show_kills);

        $public = $this->get(route('game.characters.show', ['name' => 'Alpha Knight']));
        $public->assertOk()
            ->assertSee('&lt;script&gt;alert(1)&lt;/script&gt; Owner text', false)
            ->assertDontSee('<script>alert(1)</script>', false)
            ->assertSee('Main character')
            ->assertSee('Guild details are private.')
            ->assertSee('House details are private.')
            ->assertSee('Skills are private.')
            ->assertSee('Death history is private.')
            ->assertSee('Player-kill statistics are private.')
            ->assertSee('Beta Druid')
            ->assertSee('Online');

        $this->put(route('account.characters.profile.update', ['name' => 'Beta Druid']), [
            'public_comment' => 'Secondary',
            'show_guild' => '1',
            'show_house' => '1',
            'show_skills' => '1',
            'show_deaths' => '1',
            'show_kills' => '1',
            'is_main_character' => '1',
        ])->assertRedirect();

        self::assertFalse($alpha->fresh()->is_main_character);
        self::assertTrue(CharacterProfilePreference::query()->where('canary_player_id', 2)->firstOrFail()->is_main_character);

        $this->get(route('game.characters.show', ['name' => 'Alpha Knight']))
            ->assertOk()
            ->assertDontSee('Beta Druid')
            ->assertDontSee('Main character');

        $this->get(route('account.overview'))
            ->assertOk()
            ->assertSee('Manage public profile')
            ->assertSee('Main character')
            ->assertSee('Custom profile settings');

        self::assertSame(2, DB::table('identity_security_events')
            ->where('identity_id', $identity->id)
            ->where('event_type', CharacterProfileEventRecorder::PREFERENCES_UPDATED)
            ->count());
        self::assertSame(2, DB::table('identity_security_events')
            ->where('identity_id', $identity->id)
            ->where('event_type', CharacterProfileEventRecorder::MAIN_CHARACTER_SELECTED)
            ->count());
    }

    public function test_non_owner_cannot_view_or_update_preferences(): void
    {
        $identity = $this->identityWithBinding(9001, false, false);
        $this->seedCharacters();
        $this->loginAsCurrentIdentity($identity);

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->assertNotFound();
        $this->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), [
            'public_comment' => 'Not allowed',
        ])->assertNotFound();

        self::assertSame(0, CharacterProfilePreference::query()->count());
    }

    public function test_comment_length_is_bounded(): void
    {
        $identity = $this->identityWithBinding(7001, false, false);
        $this->seedCharacters();
        $this->loginAsCurrentIdentity($identity);

        $this->from(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), [
                'public_comment' => str_repeat('a', 501),
            ])
            ->assertRedirect(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->assertSessionHasErrors('public_comment');

        self::assertSame(0, CharacterProfilePreference::query()->count());
    }

    private function seedCharacters(): void
    {
        DB::connection('canary')->table('players')->insert([
            [
                'id' => 1,
                'name' => 'Alpha Knight',
                'account_id' => 7001,
                'level' => 120,
                'vocation' => 4,
                'lastlogin' => 1_800_000_000,
                'lastlogout' => 1_799_999_000,
                'comment' => 'Canary comment',
                'deletion' => 0,
            ],
            [
                'id' => 2,
                'name' => 'Beta Druid',
                'account_id' => 7001,
                'level' => 90,
                'vocation' => 2,
                'comment' => 'Sibling comment',
                'deletion' => 0,
            ],
        ]);
        DB::connection('canary')->table('cluster_sessions')->insert([
            'id' => 1,
            'player_id' => 1,
            'channel_id' => 1,
            'status' => 'ONLINE',
            'expires_at' => ((int) floor(microtime(true) * 1000)) + 60_000,
        ]);
    }

    private function identityWithBinding(int $accountId, bool $association, bool $status): Identity
    {
        $identity = Identity::query()->create([
            'email' => uniqid('character-profile-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
            'public_account_association' => $association,
            'public_status_visible' => $status,
        ]);
        IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'canary_account_id' => $accountId,
            'provisioning_name' => 'op'.substr(hash('sha256', (string) $identity->id), 0, 30),
            'canary_creation_epoch' => 1_800_000_000 + $identity->id,
            'status' => IdentityCanaryAccount::STATUS_READY,
            'ready_at' => now(),
        ]);

        return $identity;
    }

    private function loginAsCurrentIdentity(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');
    }
}
