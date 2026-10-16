<?php

namespace Tests\Feature\PublicGameData;

use App\CharacterProfiles\Models\CharacterProfilePreference;
use App\GameProjections\PublicProjectionFamily;
use App\Identity\Models\Identity;
use App\PublicGameData\NativePublicGameData;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Redis;
use Tests\Support\SeedsNativeAccountCharacters;
use Tests\TestCase;

/** Online and highscores pages on the native public projections (removal plan packets C5 and C7). */
final class NativePublicGameDataTest extends TestCase
{
    use RefreshDatabase;
    use SeedsNativeAccountCharacters;

    private const NOW = 1_790_000_020;

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const OTHER_WORLD = '01934f10-7c02-7001-805b-3b1122334402';

    private const HERO = '01934f10-7c04-7001-805b-3b1122334401';

    private const MAGE = '01934f10-7c04-7001-805b-3b1122334402';

    private const GHOST = '01934f10-7c04-7001-805b-3b1122334403';

    protected function setUp(): void
    {
        parent::setUp();
        config([
            'game-projections.public.enabled' => true,
            'game-projections.public.identities' => ['CN=game-public-projection'],
            'game-projections.public.source_authority' => 'oteryn:game:primary',
            'public_game_data.world_id' => null,
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(self::NOW));
        DB::table('game_public_projection_state')->updateOrInsert(['id' => 1], ['highest_epoch' => '1']);
        Redis::shouldReceive('connection')->never();
        $this->enableNativeAccountCharacters();
        $this->nativeAccountCharactersLive();
    }

    /** Binds a native character to a live Platform identity through the account-character projection. */
    private function bindToIdentity(string $characterId, string $name): Identity
    {
        $identity = Identity::query()->create([
            'email' => uniqid('public-game-data-', true).'@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $this->bindAccountCharacters($identity->account_id, [[$characterId, self::WORLD, $name]]);

        return $identity;
    }

    public function test_fresh_online_projection_lists_rows_without_channel_or_identifiers(): void
    {
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->online(self::WORLD, [['Aldric', 4, '412'], ['Beren', 2, '90']]);

        $response = $this->get('/pl/online')
            ->assertOk()
            ->assertSeeInOrder(['Aldric', 'Beren'])
            ->assertSee('412')
            ->assertDontSee(self::WORLD)
            ->assertDontSee('Kanał');
        self::assertStringNotContainsString('community-unavailable', (string) $response->getContent());

        $this->get('/en/online')->assertOk()->assertSee('Aldric');
    }

    public function test_online_list_is_paginated_at_one_hundred(): void
    {
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->online(self::WORLD, array_map(fn (int $i): array => [sprintf('Online%03d', $i), 1, '10'], range(1, 101)));

        $this->get('/en/online')->assertOk()->assertSee('Online100')->assertDontSee('Online101')->assertSee('Page 1 of 2');
        $this->get('/en/online?page=2')->assertOk()->assertSee('Online101')->assertDontSee('Online001');
    }

    public function test_fresh_but_empty_online_list_is_200_with_the_empty_state(): void
    {
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->online(self::WORLD, []);

        $this->get('/en/online')
            ->assertOk()
            ->assertSee('No characters are currently online.')
            ->assertDontSee('community-unavailable', false);
        $this->get('/pl/online')->assertOk()->assertSee('Obecnie żadna postać nie jest online.');
    }

    public function test_stale_missing_or_disabled_projection_is_503_unavailable_on_both_pages(): void
    {
        // Missing watermark and no rows.
        $this->get('/pl/online')->assertStatus(503)->assertSee('community-unavailable', false);
        $this->get('/pl/highscores')->assertStatus(503)->assertSee('community-unavailable', false);

        // Rows present, watermark stale (31 s behind with 1 s clock uncertainty).
        $this->watermark(PublicProjectionFamily::WorldOnline, self::NOW - 30);
        $this->online(self::WORLD, [['Aldric', 4, '412']]);
        $this->get('/en/online')->assertStatus(503)->assertDontSee('Aldric');

        $this->watermark(PublicProjectionFamily::Highscores, self::NOW - 30);
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '11400000000']]);
        $this->get('/en/highscores')->assertStatus(503)->assertDontSee('Aldric');

        // Fresh again, but the consumer is switched off.
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->get('/en/online')->assertOk();
        config(['game-projections.public.enabled' => false]);
        $this->get('/en/online')->assertStatus(503);
    }

    public function test_watermark_below_the_highest_epoch_is_unavailable(): void
    {
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->online(self::WORLD, [['Aldric', 4, '412']]);
        DB::table('game_public_projection_state')->where('id', 1)->update(['highest_epoch' => '2']);

        $this->get('/en/online')->assertStatus(503);
    }

    public function test_highscores_join_the_profile_name_and_leave_out_rows_without_a_profile(): void
    {
        $this->watermark(PublicProjectionFamily::Highscores);
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->highscores(self::WORLD, 1, 0, [
            [self::HERO, 'OldName', 4, '412', '11400000000'],
            [self::GHOST, 'Hidden Ghost', 4, '300', '5000000'],
            [self::MAGE, 'Mira', 1, '280', '2000000'],
        ]);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->profile(self::MAGE, self::WORLD, 'Mira');
        // The ghost's profile is hidden: stored as absent.
        $this->profile(self::GHOST, self::WORLD, null);

        $response = $this->get('/en/highscores')
            ->assertOk()
            ->assertSeeInOrder(['Aldric', 'Mira'])
            ->assertDontSee('OldName')
            ->assertDontSee('Hidden Ghost')
            ->assertDontSee(self::HERO);
        // The remaining rows keep the published competition ranks 1 and 3.
        $response->assertSee('data-rank="1"', false)->assertSee('data-rank="3"', false)->assertDontSee('data-rank="2"', false);
        $this->get('/pl/highscores')->assertOk()->assertSee('Aldric');
    }

    public function test_profile_of_another_world_does_not_join(): void
    {
        $this->watermark(PublicProjectionFamily::Highscores);
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '1']]);
        $this->profile(self::HERO, self::OTHER_WORLD, 'Aldric');

        $this->get('/en/highscores')->assertOk()->assertDontSee('Aldric')->assertSee('No active characters found.');
    }

    public function test_stale_profile_family_serves_no_highscores_rows(): void
    {
        $this->watermark(PublicProjectionFamily::Highscores);
        $this->watermark(PublicProjectionFamily::CharacterProfile, self::NOW - 30);
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '1']]);
        $this->profile(self::HERO, self::WORLD, 'Aldric');

        $this->get('/en/highscores')->assertStatus(503)->assertDontSee('Aldric');
    }

    public function test_category_and_vocation_filters_select_the_published_lists(): void
    {
        $this->watermark(PublicProjectionFamily::Highscores);
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->profile(self::MAGE, self::WORLD, 'Mira');
        // Game's reference base vocations: Druid 1, Knight 2, Monk 3, Paladin 4, Sorcerer 5.
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 2, '412', '11400000000'], [self::MAGE, 'Mira', 5, '280', '2000000']]);
        $this->highscores(self::WORLD, 2, 0, [[self::MAGE, 'Mira', 5, '280', '95'], [self::HERO, 'Aldric', 2, '412', '12']]);
        $this->highscores(self::WORLD, 2, 2, [[self::HERO, 'Aldric', 2, '412', '12']]);

        $this->get('/en/highscores?category=magic')->assertOk()->assertSeeInOrder(['Mira', 'Sorcerer', 'Aldric', 'Knight'])->assertSee('95');
        // Filter n is the published list of base vocation n; 0 and no value are the all list.
        $this->get('/en/highscores?category=magic&vocation=2')->assertOk()->assertSee('Aldric')->assertDontSee('Mira');
        $this->get('/en/highscores?category=magic&vocation=0')->assertOk()->assertSee('Mira');
        $this->get('/pl/highscores?category=magic&vocation=2')->assertOk()->assertSee('Aldric')->assertSee('Knight');
        // A filter the producer did not publish is unavailable, not an empty or invented list.
        $this->get('/en/highscores?category=magic&vocation=4')->assertStatus(503);
        // Level and experience share category 1 and show level or value.
        $this->get('/en/highscores?category=level')->assertOk()->assertSee('412')->assertDontSee('11,400,000,000');
        $this->get('/en/highscores?category=experience')->assertOk()->assertSee('11,400,000,000');
        // A list the producer never published is unavailable, not an empty or invented list.
        $this->get('/en/highscores?category=fishing')->assertStatus(503);
        // Contract-invalid input is still a validation error.
        $this->get('/en/highscores?category=unsupported')->assertSessionHasErrors('category');
        $this->get('/en/highscores?vocation=9')->assertSessionHasErrors('vocation');
    }

    public function test_profile_page_shows_only_the_carried_fields(): void
    {
        $guild = '01934f10-7c06-7001-805b-3b1122334401';
        $this->bindToIdentity(self::HERO, 'Aldric');
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->watermark(PublicProjectionFamily::Guild);
        $this->snapshot(PublicProjectionFamily::CharacterProfile, self::HERO, self::WORLD, 'Aldric', true, [
            'character_id' => self::HERO,
            'profile' => [['guild' => [['guild_id' => $guild, 'rank_level' => 3]], 'level' => '412', 'name' => 'Aldric', 'vocation' => 2, 'world_id' => self::WORLD, 'guild_visible' => true]],
        ]);
        $this->snapshot(PublicProjectionFamily::Guild, $guild, self::WORLD, 'Vanguard', true, [
            'guild' => [['founded_at' => '1', 'members' => [], 'name' => 'Vanguard', 'ranks' => [['level' => 1, 'name' => 'Recruit'], ['level' => 3, 'name' => 'Captain'], ['level' => 20, 'name' => 'Leader']], 'state' => 'ACTIVE', 'world_id' => self::WORLD]],
            'guild_id' => $guild,
        ]);
        $this->profile(self::MAGE, self::WORLD, 'Mira');

        $this->get('/en/characters/Aldric')
            ->assertOk()
            ->assertSee('Aldric')
            ->assertSee('412')
            ->assertSee('Knight')
            ->assertSee('Vanguard')
            ->assertSee('Captain')
            ->assertDontSee(self::HERO)
            ->assertDontSee(self::WORLD)
            ->assertDontSee('Skills')
            ->assertDontSee('Recent deaths')
            ->assertDontSee('Magic level');
        // The name resolves ASCII case-folded.
        $this->get('/en/characters/aldric')->assertOk()->assertSee('Aldric');
        $this->get('/pl/characters/Mira')->assertOk()->assertSee('Mira')->assertSee('Brak gildii');
        $this->get('/en/characters/Nobody')->assertNotFound();
    }

    public function test_profile_page_hides_unpublished_or_other_world_characters_and_fails_closed(): void
    {
        $this->bindToIdentity(self::HERO, 'Aldric');
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->profile(self::GHOST, self::WORLD, null);
        $this->profile(self::MAGE, self::OTHER_WORLD, 'Mira');
        config(['public_game_data.world_id' => self::WORLD]);

        $this->get('/en/characters/Aldric')->assertOk();
        // A hidden character (profile []) and a character of another World are not found.
        $this->get('/en/characters/Mira')->assertNotFound();

        // A member of a guild the guild family cannot name shows no guild instead of a wrong one.
        $this->snapshot(PublicProjectionFamily::CharacterProfile, self::HERO, self::WORLD, 'Aldric', true, [
            'character_id' => self::HERO,
            'profile' => [['guild' => [['guild_id' => '01934f10-7c06-7001-805b-3b1122334409', 'rank_level' => 3]], 'level' => '412', 'name' => 'Aldric', 'vocation' => 2, 'world_id' => self::WORLD, 'guild_visible' => true]],
        ]);
        $this->get('/en/characters/Aldric')->assertOk()->assertDontSee('No guild');

        // guild_visible=false hides the guild row altogether: no name, no rank, no "No guild".
        $this->snapshot(PublicProjectionFamily::CharacterProfile, self::HERO, self::WORLD, 'Aldric', true, [
            'character_id' => self::HERO,
            'profile' => [['guild' => [['guild_id' => '01934f10-7c06-7001-805b-3b1122334401', 'rank_level' => 3]], 'guild_visible' => false, 'level' => '412', 'name' => 'Aldric', 'vocation' => 2, 'world_id' => self::WORLD]],
        ]);
        $this->snapshot(PublicProjectionFamily::Guild, '01934f10-7c06-7001-805b-3b1122334401', self::WORLD, 'Vanguard', true, [
            'guild' => [['founded_at' => '1', 'members' => [], 'name' => 'Vanguard', 'ranks' => [['level' => 1, 'name' => 'Recruit'], ['level' => 3, 'name' => 'Captain'], ['level' => 20, 'name' => 'Leader']], 'state' => 'ACTIVE', 'world_id' => self::WORLD]],
            'guild_id' => '01934f10-7c06-7001-805b-3b1122334401',
        ]);
        $this->watermark(PublicProjectionFamily::Guild);
        $this->get('/en/characters/Aldric')->assertOk()->assertSee('Aldric')
            ->assertDontSee('Vanguard')->assertDontSee('Captain')->assertDontSee('No guild');

        // A stale or missing family is unavailable (503), never a guess or a legacy read.
        $this->watermark(PublicProjectionFamily::CharacterProfile, self::NOW - 31);
        $this->get('/en/characters/Aldric')->assertStatus(503)->assertSee('community-unavailable', false);
        DB::table('game_public_projection_watermarks')->delete();
        $this->get('/en/characters/Aldric')->assertStatus(503);
    }

    public function test_world_is_resolved_from_config_or_the_only_published_world(): void
    {
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->online(self::WORLD, [['Aldric', 4, '412']]);
        $this->online(self::OTHER_WORLD, [['Zed', 1, '8']]);

        // Two worlds and no configured one fail closed instead of guessing.
        $this->get('/en/online')->assertStatus(503);

        config(['public_game_data.world_id' => self::OTHER_WORLD]);
        $this->get('/en/online')->assertOk()->assertSee('Zed')->assertDontSee('Aldric');

        config(['public_game_data.world_id' => 'not-a-world']);
        $this->get('/en/online')->assertStatus(503);
    }

    private function guildedHero(): void
    {
        $guild = '01934f10-7c06-7001-805b-3b1122334401';
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->watermark(PublicProjectionFamily::Guild);
        $this->snapshot(PublicProjectionFamily::CharacterProfile, self::HERO, self::WORLD, 'Aldric', true, [
            'character_id' => self::HERO,
            'profile' => [['guild' => [['guild_id' => $guild, 'rank_level' => 3]], 'level' => '412', 'name' => 'Aldric', 'vocation' => 2, 'world_id' => self::WORLD, 'guild_visible' => true]],
        ]);
        $this->snapshot(PublicProjectionFamily::Guild, $guild, self::WORLD, 'Vanguard', true, [
            'guild' => [['founded_at' => '1', 'members' => [], 'name' => 'Vanguard', 'ranks' => [['level' => 1, 'name' => 'Recruit'], ['level' => 3, 'name' => 'Captain'], ['level' => 20, 'name' => 'Leader']], 'state' => 'ACTIVE', 'world_id' => self::WORLD]],
            'guild_id' => $guild,
        ]);
    }

    public function test_owner_preference_is_an_upper_bound_over_the_game_guild_flag(): void
    {
        $identity = $this->bindToIdentity(self::HERO, 'Aldric');
        $this->guildedHero();

        // Both allow (no saved preference keeps the default): shown.
        $this->get('/en/characters/Aldric')->assertOk()->assertSee('Vanguard')->assertSee('Captain');

        // The owner opts out: hidden, with no "No guild" either.
        $preference = CharacterProfilePreference::query()->create(['identity_id' => $identity->id, 'character_id' => self::HERO, 'show_guild' => false]);
        $this->get('/en/characters/Aldric')->assertOk()->assertDontSee('Vanguard')->assertDontSee('Captain')->assertDontSee('No guild');

        // The owner opts in again: shown.
        $preference->forceFill(['show_guild' => true])->save();
        $this->get('/en/characters/Aldric')->assertOk()->assertSee('Vanguard');

        // The preference never widens Game's flag: guild_visible=false stays hidden.
        $this->snapshot(PublicProjectionFamily::CharacterProfile, self::HERO, self::WORLD, 'Aldric', true, [
            'character_id' => self::HERO,
            'profile' => [['guild' => [['guild_id' => '01934f10-7c06-7001-805b-3b1122334401', 'rank_level' => 3]], 'guild_visible' => false, 'level' => '412', 'name' => 'Aldric', 'vocation' => 2, 'world_id' => self::WORLD]],
        ]);
        $this->get('/en/characters/Aldric')->assertOk()->assertDontSee('Vanguard');
    }

    public function test_a_character_that_cannot_be_bound_to_an_identity_hides_the_guild(): void
    {
        $this->guildedHero();

        $this->get('/en/characters/Aldric')->assertOk()->assertSee('Aldric')->assertDontSee('Vanguard')->assertDontSee('Captain');
    }

    public function test_name_held_in_two_worlds_or_by_a_case_variant_is_not_found(): void
    {
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->get('/en/characters/Aldric')->assertOk();

        // The same name in another World: ambiguous even though the served World holds exactly one.
        $this->profile(self::MAGE, self::OTHER_WORLD, 'Aldric');
        config(['public_game_data.world_id' => self::WORLD]);
        $this->get('/en/characters/Aldric')->assertNotFound();
        $this->get('/en/characters/aldric')->assertNotFound();
        // Without a configured World, two published Worlds fail closed instead of guessing.
        config(['public_game_data.world_id' => null]);
        $this->get('/en/characters/Aldric')->assertStatus(503);
    }

    public function test_case_variant_holder_makes_the_name_ambiguous(): void
    {
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->profile(self::MAGE, self::WORLD, 'ALDRIC');

        $this->get('/en/characters/Aldric')->assertNotFound();
        $this->get('/en/characters/ALDRIC')->assertNotFound();
        $this->get('/en/characters/aldric')->assertNotFound();
    }

    public function test_the_only_holder_in_the_served_world_resolves_when_another_world_holds_the_name_hidden(): void
    {
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        // A hidden profile is stored as absent and does not count as a holder.
        $this->profile(self::GHOST, self::OTHER_WORLD, null);
        config(['public_game_data.world_id' => self::WORLD]);

        $this->get('/en/characters/Aldric')->assertOk();
    }

    public function test_unknown_name_and_hidden_profile_have_identical_bodies(): void
    {
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->profile(self::GHOST, self::WORLD, null);

        $unknown = $this->get('/en/characters/Nobody')->assertNotFound();
        $hidden = $this->get('/en/characters/Hidden%20Ghost')->assertNotFound();
        self::assertSame($unknown->getContent(), $hidden->getContent());
    }

    public function test_entry_at_an_older_epoch_is_not_served_while_the_watermark_is_at_the_highest_epoch(): void
    {
        DB::table('game_public_projection_state')->where('id', 1)->update(['highest_epoch' => '2']);
        foreach ([PublicProjectionFamily::WorldOnline, PublicProjectionFamily::Highscores, PublicProjectionFamily::CharacterProfile] as $family) {
            DB::table('game_public_projection_watermarks')->updateOrInsert(['family' => $family->value], [
                'source_authority' => 'oteryn:game:primary',
                'projection_epoch' => '2',
                'complete_through' => self::NOW,
                'observed_at' => self::NOW,
                'created_at' => now(),
                'updated_at' => now(),
            ]);
        }
        // The rows were published at epoch 1 and never re-published after the epoch change.
        $this->online(self::WORLD, [['Aldric', 4, '412']]);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '1']]);

        $this->get('/en/online')->assertStatus(503)->assertDontSee('Aldric');
        $this->get('/en/characters/Aldric')->assertStatus(503)->assertDontSee('412');
        $this->get('/en/highscores')->assertStatus(503)->assertDontSee('Aldric');
    }

    public function test_configured_world_missing_from_the_data_is_503_not_404(): void
    {
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->online(self::WORLD, [['Aldric', 4, '412']]);
        config(['public_game_data.world_id' => self::OTHER_WORLD]);

        $this->get('/en/characters/Aldric')->assertStatus(503)->assertSee('community-unavailable', false);
        $this->get('/en/characters/Nobody')->assertStatus(503);
        $this->get('/en/online')->assertStatus(503);
    }

    public function test_highscores_and_profile_are_503_with_the_consumer_switched_off(): void
    {
        $this->watermark(PublicProjectionFamily::Highscores);
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '1']]);
        $this->get('/en/highscores')->assertOk();
        $this->get('/en/characters/Aldric')->assertOk();

        config(['game-projections.public.enabled' => false]);
        $this->get('/en/highscores')->assertStatus(503)->assertDontSee('Aldric');
        $this->get('/en/characters/Aldric')->assertStatus(503)->assertDontSee('Druid');
    }

    public function test_pages_are_never_stored_by_a_cache_including_error_responses(): void
    {
        $this->watermark(PublicProjectionFamily::WorldOnline);
        $this->watermark(PublicProjectionFamily::Highscores);
        $this->watermark(PublicProjectionFamily::CharacterProfile);
        $this->online(self::WORLD, [['Aldric', 4, '412']]);
        $this->profile(self::HERO, self::WORLD, 'Aldric');
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '1']]);

        foreach (['/en/online', '/en/highscores', '/en/characters/Aldric', '/en/characters/Nobody', '/pl/online'] as $url) {
            $cache = (string) $this->get($url)->headers->get('Cache-Control');
            self::assertStringContainsString('no-store', $cache, $url);
            self::assertStringContainsString('private', $cache, $url);
        }
        // A 503 carries it too.
        DB::table('game_public_projection_watermarks')->delete();
        $cache = (string) $this->get('/en/online')->assertStatus(503)->headers->get('Cache-Control');
        self::assertStringContainsString('no-store', $cache);
    }

    public function test_public_pages_are_rate_limited_per_ip(): void
    {
        for ($i = 0; $i < 120; $i++) {
            $this->get('/en/online');
        }
        $limited = $this->get('/en/online')->assertStatus(429);
        // The 429 is not stored either: no-store wraps the throttle.
        self::assertStringContainsString('no-store', (string) $limited->headers->get('Cache-Control'));
        // The budget is shared by the three pages and keyed on the client IP.
        $this->get('/en/highscores')->assertStatus(429);
        $this->get('/en/characters/Aldric')->assertStatus(429);
        $this->withServerVariables(['REMOTE_ADDR' => '203.0.113.9'])->get('/en/online')->assertStatus(503);
    }

    public function test_uint64_counters_saturate_only_above_php_int_max(): void
    {
        $count = new \ReflectionMethod(NativePublicGameData::class, 'count');
        $reader = app(NativePublicGameData::class);

        self::assertSame(0, $count->invoke($reader, '0'));
        self::assertSame(999_999_999_999_999_999, $count->invoke($reader, '999999999999999999'));
        self::assertSame(1_000_000_000_000_000_000, $count->invoke($reader, '1000000000000000000'));
        self::assertSame(PHP_INT_MAX, $count->invoke($reader, (string) PHP_INT_MAX));
        self::assertSame(PHP_INT_MAX, $count->invoke($reader, '9223372036854775808'));
        self::assertSame(PHP_INT_MAX, $count->invoke($reader, '18446744073709551615'));
        self::assertSame(0, $count->invoke($reader, '01'));
        self::assertSame(0, $count->invoke($reader, 7));
    }

    private function watermark(PublicProjectionFamily $family, int $completeThrough = self::NOW): void
    {
        DB::table('game_public_projection_watermarks')->updateOrInsert(['family' => $family->value], [
            'source_authority' => 'oteryn:game:primary',
            'projection_epoch' => '1',
            'complete_through' => $completeThrough,
            'observed_at' => $completeThrough,
            'created_at' => now(),
            'updated_at' => now(),
        ]);
    }

    /** @param  list<array{string, int, string}>  $characters */
    private function online(string $world, array $characters): void
    {
        $this->snapshot(PublicProjectionFamily::WorldOnline, $world, $world, null, true, [
            'characters' => array_map(fn (array $c): array => ['level' => $c[2], 'name' => $c[0], 'vocation' => $c[1]], $characters),
            'world_id' => $world,
        ]);
    }

    /** @param  list<array{string, string, int, string, string}>  $rows character, row name, vocation, level, value */
    private function highscores(string $world, int $category, int $filter, array $rows): void
    {
        $position = 0;
        $this->snapshot(PublicProjectionFamily::Highscores, $world.':'.$category.':'.$filter, $world, null, true, [
            'category' => $category,
            'computed_at' => '1790000000',
            'filter' => $filter,
            'rows' => array_map(function (array $row) use (&$position): array {
                $position++;

                return [
                    'character_id' => $row[0],
                    'level' => $row[3],
                    'name' => $row[1],
                    'rank' => (string) ($row[0] === self::MAGE && $position === 3 ? 3 : $position),
                    'value' => $row[4],
                    'vocation' => $row[2],
                ];
            }, $rows),
            'world_id' => $world,
        ]);
    }

    private function profile(string $character, string $world, ?string $name): void
    {
        $this->snapshot(PublicProjectionFamily::CharacterProfile, $character, $world, $name, $name !== null, [
            'character_id' => $character,
            'profile' => $name === null ? [] : [['guild' => [], 'level' => '100', 'name' => $name, 'vocation' => 1, 'world_id' => $world, 'guild_visible' => true]],
        ]);
    }

    /** @param  array<string, mixed>  $payload */
    private function snapshot(PublicProjectionFamily $family, string $subject, ?string $world, ?string $lookup, bool $present, array $payload): void
    {
        $json = json_encode($payload, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR);
        DB::table('game_public_projection_snapshots')->updateOrInsert(
            ['family' => $family->value, 'subject_key' => $subject],
            [
                'world_id' => $world,
                'lookup_name' => $lookup,
                'lookup_folded' => $lookup === null ? null : strtolower($lookup),
                'present' => $present,
                'source_authority' => 'oteryn:game:primary',
                'projection_epoch' => '1',
                'projection_revision' => '1',
                'source_observed_at' => self::NOW,
                'content_digest' => hash('sha256', $json),
                'payload' => $json,
                'created_at' => now(),
                'updated_at' => now(),
            ],
        );
    }
}
