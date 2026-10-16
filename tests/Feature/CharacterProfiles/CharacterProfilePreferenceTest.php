<?php

namespace Tests\Feature\CharacterProfiles;

use App\CharacterProfiles\CharacterGuildVisibility;
use App\CharacterProfiles\CharacterProfileEventRecorder;
use App\CharacterProfiles\CharacterProfilePreferenceService;
use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Schema;
use PHPUnit\Framework\Attributes\DataProvider;
use Tests\Support\SeedsNativeAccountCharacters;
use Tests\TestCase;

/** Preferences keyed by the native CharacterId, owned through the account-character projection. */
final class CharacterProfilePreferenceTest extends TestCase
{
    use RefreshDatabase;
    use SeedsNativeAccountCharacters;

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const ALPHA = '01934f10-7c04-7001-805b-3b1122334411';

    private const BETA = '01934f10-7c04-7001-805b-3b1122334412';

    private const STRANGER = '01934f10-7c04-7001-805b-3b1122334499';

    protected function setUp(): void
    {
        parent::setUp();
        $this->enableNativeAccountCharacters();
        $this->nativeAccountCharactersLive();
    }

    public function test_guest_cannot_manage_character_profile_preferences(): void
    {
        $this->get('/account/characters/Alpha/profile')->assertRedirect('/login');
        $this->put('/account/characters/Alpha/profile')->assertRedirect('/login');
    }

    public function test_owner_saves_the_guild_toggle_and_main_character_by_native_character_id(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight'], [self::BETA, self::WORLD, 'Beta Druid']]);
        $this->login($identity);

        $this->get(route('account.characters.profile.edit', ['name' => 'alpha knight']))
            ->assertOk()
            ->assertSee('Alpha Knight');

        $this->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), [
            'is_main_character' => '1',
        ])->assertRedirect(route('account.characters.profile.edit', ['name' => 'Alpha Knight']));

        $alpha = CharacterProfilePreference::query()->where('character_id', self::ALPHA)->sole();
        self::assertSame($identity->id, $alpha->identity_id);
        self::assertNull($alpha->canary_player_id);
        self::assertTrue($alpha->is_main_character);
        self::assertFalse($alpha->show_guild);

        $this->put(route('account.characters.profile.update', ['name' => 'Beta Druid']), [
            'show_guild' => '1',
            'is_main_character' => '1',
        ])->assertRedirect();

        self::assertFalse($alpha->refresh()->is_main_character);
        $beta = CharacterProfilePreference::query()->where('character_id', self::BETA)->sole();
        self::assertTrue($beta->is_main_character);
        self::assertTrue($beta->show_guild);

        self::assertSame(2, DB::table('identity_security_events')
            ->where('identity_id', $identity->id)
            ->where('event_type', CharacterProfileEventRecorder::PREFERENCES_UPDATED)
            ->count());
        self::assertSame(2, DB::table('identity_security_events')
            ->where('identity_id', $identity->id)
            ->where('event_type', CharacterProfileEventRecorder::MAIN_CHARACTER_SELECTED)
            ->count());
    }

    public function test_form_ignores_fields_that_no_longer_map_to_anything(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $this->login($identity);

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->assertOk()
            ->assertDontSee('name="public_comment"', false)
            ->assertDontSee('name="show_skills"', false)
            ->assertDontSee('name="show_deaths"', false)
            ->assertDontSee('name="show_house"', false);

        $this->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), [
            'public_comment' => 'ignored',
            'show_skills' => '1',
            'show_guild' => '1',
        ])->assertRedirect();

        $row = CharacterProfilePreference::query()->where('character_id', self::ALPHA)->sole();
        self::assertNull($row->public_comment);
        self::assertTrue($row->show_guild);
    }

    public function test_guild_preference_is_an_upper_bound_and_unbound_characters_fail_closed(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $visibility = app(CharacterGuildVisibility::class);

        // Bound, no saved preference: the default is visible.
        self::assertTrue($visibility->allows(self::ALPHA));

        $preference = CharacterProfilePreference::query()->create([
            'identity_id' => $identity->id,
            'character_id' => self::ALPHA,
            'show_guild' => false,
        ]);
        self::assertFalse($visibility->allows(self::ALPHA));

        $preference->forceFill(['show_guild' => true])->save();
        self::assertTrue($visibility->allows(self::ALPHA));

        // A character no account binds to hides the guild.
        self::assertFalse($visibility->allows(self::STRANGER));

        // A disabled identity hides it too.
        $identity->forceFill(['disabled_at' => now()])->save();
        self::assertFalse($visibility->allows(self::ALPHA));
    }

    /** @return iterable<string, array{string}> */
    public static function unprovenOwnership(): iterable
    {
        foreach (['disabled', 'stale', 'watermark-behind', 'epoch-fenced', 'invalid-snapshot', 'ambiguous'] as $state) {
            yield $state => [$state];
        }
    }

    #[DataProvider('unprovenOwnership')]
    public function test_guild_is_hidden_when_live_ownership_is_not_proven(string $state): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        CharacterProfilePreference::query()->create([
            'identity_id' => $identity->id,
            'character_id' => self::ALPHA,
            'show_guild' => true,
        ]);
        $visibility = app(CharacterGuildVisibility::class);
        self::assertTrue($visibility->allows(self::ALPHA));

        match ($state) {
            // The consumer is switched off while the old rows stay in the table.
            'disabled' => config(['game-auth.native_account_characters.enabled' => false]),
            'stale' => DB::table('native_account_character_projection_state')->where('id', 1)->update(['complete_through' => now()->getTimestamp() - 120]),
            'watermark-behind' => DB::table('native_account_character_projection_state')->where('id', 1)->update(['highest_epoch' => '2']),
            'epoch-fenced' => DB::table('native_account_character_projection_state')->where('id', 1)->update(['highest_epoch' => '2', 'watermark_epoch' => '2']),
            'invalid-snapshot' => DB::table('native_account_character_snapshots')->where('account_id', $identity->account_id)->update(['invalid' => true]),
            'ambiguous' => $this->bindAccountCharacters($this->identity()->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]),
            default => throw new \LogicException('Unknown ownership state.'),
        };

        self::assertSame(1, DB::table('native_account_character_rows')->where('account_id', $identity->account_id)->where('character_id', self::ALPHA)->count());
        self::assertFalse($visibility->allows(self::ALPHA));
    }

    public function test_leftover_row_at_a_fenced_epoch_does_not_block_the_live_owner(): void
    {
        $previous = $this->identity();
        $this->bindAccountCharacters($previous->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $owner = $this->identity();
        $this->bindAccountCharacters($owner->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        DB::table('native_account_character_projection_state')->where('id', 1)->update(['highest_epoch' => '2', 'watermark_epoch' => '2']);
        DB::table('native_account_character_snapshots')->where('account_id', $owner->account_id)->update(['projection_epoch' => '2']);
        CharacterProfilePreference::query()->create([
            'identity_id' => $previous->id,
            'character_id' => self::ALPHA,
            'show_guild' => true,
        ]);
        CharacterProfilePreference::query()->create([
            'identity_id' => $owner->id,
            'character_id' => self::ALPHA,
            'show_guild' => false,
        ]);

        // Only the owner at the live epoch decides; the fenced account's preference is inert.
        self::assertFalse(app(CharacterGuildVisibility::class)->allows(self::ALPHA));
    }

    public function test_non_owner_cannot_view_or_update_preferences(): void
    {
        $owner = $this->identity();
        $this->bindAccountCharacters($owner->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $other = $this->identity();
        $this->bindAccountCharacters($other->account_id, [[self::BETA, self::WORLD, 'Beta Druid']]);
        $this->login($other);

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))->assertNotFound();
        $this->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), ['show_guild' => '1'])->assertNotFound();

        self::assertSame(0, CharacterProfilePreference::query()->count());
    }

    public function test_identity_without_a_projection_snapshot_is_not_found(): void
    {
        $this->login($this->identity());

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))->assertNotFound();
    }

    public function test_stale_projection_is_a_sanitized_503_and_writes_nothing(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $this->login($identity);
        DB::table('native_account_character_projection_state')->where('id', 1)->update(['complete_through' => now()->getTimestamp() - 120]);

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->assertStatus(503)
            ->assertDontSee('SQLSTATE');
        $this->put(route('account.characters.profile.update', ['name' => 'Alpha Knight']), ['show_guild' => '1'])
            ->assertSessionHasErrors('profile');

        self::assertSame(0, CharacterProfilePreference::query()->count());
    }

    public function test_case_variant_names_resolve_only_by_exact_name(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Aldric'], [self::BETA, self::WORLD, 'ALDRIC']]);
        $this->login($identity);

        $this->put(route('account.characters.profile.update', ['name' => 'ALDRIC']), ['show_guild' => '1'])->assertRedirect();
        self::assertSame(self::BETA, CharacterProfilePreference::query()->sole()->character_id);

        $this->put(route('account.characters.profile.update', ['name' => 'Aldric']), ['is_main_character' => '1'])->assertRedirect();
        self::assertTrue(CharacterProfilePreference::query()->where('character_id', self::ALPHA)->sole()->is_main_character);
        self::assertTrue(CharacterProfilePreference::query()->where('character_id', self::BETA)->sole()->show_guild);

        // A folded name matching both is ambiguous: not found, nothing written.
        $this->get(route('account.characters.profile.edit', ['name' => 'aldric']))->assertNotFound();
        $this->put(route('account.characters.profile.update', ['name' => 'aldric']), ['show_guild' => '1'])->assertNotFound();
        self::assertSame(2, CharacterProfilePreference::query()->count());
    }

    public function test_a_name_with_a_slash_reaches_its_preference_form(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha/Knight']]);
        $this->login($identity);

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha/Knight']))->assertOk();
        $this->put(route('account.characters.profile.update', ['name' => 'Alpha/Knight']), ['show_guild' => '1'])->assertRedirect();
        self::assertSame(self::ALPHA, CharacterProfilePreference::query()->sole()->character_id);
    }

    public function test_edit_form_starts_from_an_unbound_legacy_guild_opt_out(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $service = app(CharacterProfilePreferenceService::class);

        self::assertTrue($service->editState($identity, 'Alpha Knight')['preference']->show_guild);

        CharacterProfilePreference::query()->create(['identity_id' => $identity->id, 'character_id' => null, 'show_guild' => false]);
        self::assertFalse($service->editState($identity, 'Alpha Knight')['preference']->show_guild);
    }

    public function test_preference_read_failure_is_a_sanitized_503(): void
    {
        $identity = $this->identity();
        $this->bindAccountCharacters($identity->account_id, [[self::ALPHA, self::WORLD, 'Alpha Knight']]);
        $this->login($identity);
        Schema::rename('character_profile_preferences', 'character_profile_preferences_gone');

        $this->get(route('account.characters.profile.edit', ['name' => 'Alpha Knight']))
            ->assertStatus(503)
            ->assertDontSee('SQLSTATE');
    }

    private function identity(): Identity
    {
        return Identity::query()->create([
            'email' => uniqid('character-profile-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function login(Identity $identity): void
    {
        $this->post('/login', [
            'email' => $identity->email,
            'password' => 'Correct-Horse-9!Battery',
        ])->assertRedirect(route('home'));
        $this->assertAuthenticatedAs($identity, 'web');
    }
}
