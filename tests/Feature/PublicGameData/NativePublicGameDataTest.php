<?php

namespace Tests\Feature\PublicGameData;

use App\GameProjections\PublicProjectionFamily;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Redis;
use Tests\TestCase;

/** Online and highscores pages on the native public projections (removal plan packets C5 and C7). */
final class NativePublicGameDataTest extends TestCase
{
    use RefreshDatabase;

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
        $this->highscores(self::WORLD, 1, 0, [[self::HERO, 'Aldric', 4, '412', '11400000000'], [self::MAGE, 'Mira', 1, '280', '2000000']]);
        // Magic level (2), all vocations and knights (filter 4) with a promoted knight (8).
        $this->highscores(self::WORLD, 2, 0, [[self::MAGE, 'Mira', 1, '280', '95'], [self::HERO, 'Aldric', 8, '412', '12']]);
        $this->highscores(self::WORLD, 2, 4, [[self::HERO, 'Aldric', 8, '412', '12']]);

        $this->get('/en/highscores?category=magic')->assertOk()->assertSeeInOrder(['Mira', 'Aldric'])->assertSee('95');
        $this->get('/en/highscores?category=magic&vocation=4')->assertOk()->assertDontSee('Mira');
        // The base list of knights holds a promoted knight (8): selecting elite knight keeps only him,
        // selecting knight keeps only exact knights.
        $this->get('/en/highscores?category=magic&vocation=8')->assertOk()->assertSee('Aldric')->assertDontSee('Mira');
        $this->get('/en/highscores?category=magic&vocation=4')->assertOk()->assertDontSee('Aldric');
        // Level and experience share category 1 and show level or value.
        $this->get('/en/highscores?category=level')->assertOk()->assertSee('412')->assertDontSee('11,400,000,000');
        $this->get('/en/highscores?category=experience')->assertOk()->assertSee('11,400,000,000');
        // A list the producer never published is unavailable, not an empty or invented list.
        $this->get('/en/highscores?category=fishing')->assertStatus(503);
        // Contract-invalid input is still a validation error.
        $this->get('/en/highscores?category=unsupported')->assertSessionHasErrors('category');
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
            'profile' => $name === null ? [] : [['guild' => [], 'level' => '100', 'name' => $name, 'vocation' => 1, 'world_id' => $world]],
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
