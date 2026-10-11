<?php

namespace Tests\Feature\GameProjections;

use App\GameProjections\PublicProjectionFamily;
use App\GameProjections\PublicProjectionReadModel;
use App\GameProjections\PublicProjectionSettings;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/** Game Public Projections v1 consumer ingest (removal plan packet C4, contract §3-§6, §9). */
final class PublicProjectionIngestTest extends TestCase
{
    use RefreshDatabase;

    private const IDENTITY = 'CN=game-public-projection';

    private const LCFA_IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:game:primary';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const GUILD = '01934f10-7c06-7001-805b-3b1122334401';

    /** Wire examples copied verbatim from the Game contract §4.1-§4.6 and §5.1. */
    private const CONTRACT_FIXTURES = [
        'highscores' => '{"contract_version":1,"operation":"PublishWorldHighscoresV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","category":1,"filter":0,"projection_epoch":"1","projection_revision":"17","source_observed_at":"1790000000","computed_at":"1790000000","rows":[{"rank":"1","character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","value":"11400000000"}]}',
        'character_profile' => '{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"9","source_observed_at":"1790000000","profile":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","guild":[{"guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":3}]}]}',
        'character_deaths' => '{"contract_version":1,"operation":"PublishCharacterDeathsV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790000000","deaths":[{"occurred_at":"1789990000","level":"413","pvp":true,"killers":["Beren","Mira"]}]}',
        'guild' => '{"contract_version":1,"operation":"PublishGuildV1","source_authority":"oteryn:game:primary","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"5","source_observed_at":"1790000000","guild":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Red Rose","state":"ACTIVE","founded_at":"1789000000","ranks":[{"level":1,"name":"Leader"},{"level":2,"name":"Vice Leader"},{"level":3,"name":"Member"}],"members":[{"name":"Aldric","rank_level":1,"joined_at":"1789000000"}]}]}',
        'house' => '{"contract_version":1,"operation":"PublishHouseV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","house_key":"oteryn:content.house.example-street-1","projection_epoch":"1","projection_revision":"12","source_observed_at":"1790000000","house":[{"name":"Example Street 1","kind":"private_house","town":"oteryn:content.area.city.example","size_sqm":"24","beds":"2","rent_gold":"5000","status":"OWNED","owner_kind":"CHARACTER","owner_name":"Aldric","paid_until":"1792000000"}]}',
        'world_online' => '{"contract_version":1,"operation":"PublishWorldOnlineV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8812","source_observed_at":"1790000000","characters":[{"name":"Aldric","vocation":4,"level":"412"}]}',
        'watermark' => '{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_online","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}',
    ];

    protected function setUp(): void
    {
        parent::setUp();
        config([
            'game-projections.public.enabled' => true,
            'game-projections.public.identities' => [self::IDENTITY],
            'game-projections.public.source_authority' => self::AUTHORITY,
            'game-projections.public.requests_per_minute' => 6000,
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::LCFA_IDENTITY],
            'game-auth.native_account_characters.source_authority' => 'oteryn:character-authority:primary',
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_020));
    }

    public function test_every_contract_fixture_is_accepted_with_exact_ack_and_no_store(): void
    {
        foreach (PublicProjectionFamily::cases() as $family) {
            self::assertSame(self::CONTRACT_FIXTURES[$family->value], $this->snapshot($family), $family->value);
            $response = $this->publish($family->path(), $this->snapshot($family));
            $response->assertOk();
            self::assertSame('{"contract_version":1,"result":"accepted"}', $response->getContent(), $family->value);
            self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        }
        self::assertSame(6, DB::table('game_public_projection_snapshots')->count());

        self::assertSame(self::CONTRACT_FIXTURES['watermark'], $this->watermark(PublicProjectionFamily::WorldOnline));
        $this->publish('watermark', $this->watermark(PublicProjectionFamily::WorldOnline))
            ->assertOk()
            ->assertContent('{"contract_version":1,"result":"accepted"}');

        $online = $this->readModel()->entry(PublicProjectionFamily::WorldOnline, self::WORLD, now()->getTimestamp());
        self::assertIsArray($online);
        self::assertSame([['level' => '412', 'name' => 'Aldric', 'vocation' => 4]], $online['characters']);
    }

    public function test_house_status_member_sets_and_zero_beds(): void
    {
        $base = ['name' => 'Example Shop 1', 'kind' => 'shop', 'town' => 'oteryn:content.area.city.example', 'size_sqm' => '24', 'beds' => '0', 'rent_gold' => '5000'];
        $accepted = [
            ['status' => 'VACANT'],
            ['status' => 'AUCTION', 'current_bid' => '0', 'auction_ends_at' => '1790100000'],
            ['status' => 'MOVE_OUT_PENDING', 'owner_kind' => 'GUILD', 'owner_name' => 'Red Rose', 'paid_until' => '1792000000', 'move_out_at' => '1790500000'],
            ['status' => 'UNAVAILABLE'],
        ];
        foreach ($accepted as $index => $status) {
            $this->publish('house', $this->snapshot(PublicProjectionFamily::House, ['projection_revision' => (string) (20 + $index)], [$base + $status]))
                ->assertOk();
        }

        foreach ([
            ['status' => 'VACANT', 'owner_name' => 'Aldric'],
            ['status' => 'OWNED', 'owner_kind' => 'CHARACTER', 'owner_name' => 'Aldric'],
            ['status' => 'AUCTION', 'current_bid' => '0'],
            ['status' => 'DISPOSITION'],
        ] as $status) {
            $this->assertEmptyFailure($this->publish('house', $this->snapshot(PublicProjectionFamily::House, ['projection_revision' => '99'], [$base + $status])), 400);
        }
        $this->assertEmptyFailure($this->publish('house', $this->snapshot(PublicProjectionFamily::House, ['projection_revision' => '99'], [['rent_gold' => '0'] + $base + ['status' => 'VACANT']])), 400);
    }

    public function test_routes_accept_only_the_dedicated_public_projection_identity(): void
    {
        $wire = $this->snapshot(PublicProjectionFamily::CharacterProfile);
        $this->assertEmptyFailure($this->publish('character-profile', $wire, 'CN=other'), 401);
        $this->assertEmptyFailure($this->publish('character-profile', $wire, self::LCFA_IDENTITY), 401);
        $this->assertEmptyFailure($this->publish('character-profile', $wire, self::IDENTITY, ['SSL_PROTOCOL' => 'TLSv1.2']), 401);
        $this->assertEmptyFailure($this->publish('character-profile', $wire, self::IDENTITY, ['SSL_CLIENT_VERIFY' => 'FAILED']), 401);
        $this->assertEmptyFailure($this->publish('watermark', $this->watermark(PublicProjectionFamily::Guild), 'CN=other'), 401);

        // The public-projection identity is refused on another internal route (LCFA).
        $this->assertEmptyFailure($this->call('POST', '/internal/v1/game-auth/native-account-characters/watermark', [], [], [], $this->server([
            'SSL_CLIENT_S_DN' => self::IDENTITY,
        ]), '{}'), 401);

        // A source authority other than the configured one is the wrong identity for the route.
        $this->assertEmptyFailure($this->publish('character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile, ['source_authority' => 'oteryn:game:other'])), 401);
        self::assertSame(0, DB::table('game_public_projection_snapshots')->count());

        // Reusing an identity of another internal purpose disables the whole consumer.
        config(['game-projections.public.identities' => [self::IDENTITY, self::LCFA_IDENTITY]]);
        self::assertNull(PublicProjectionSettings::current());
        $this->assertEmptyFailure($this->publish('character-profile', $wire), 503);
    }

    public function test_consumer_is_unavailable_when_disabled_or_outside_testing_and_preproduction(): void
    {
        config(['game-projections.public.enabled' => 'true']);
        $this->assertEmptyFailure($this->publish('guild', $this->snapshot(PublicProjectionFamily::Guild)), 503);

        config(['game-projections.public.enabled' => true]);
        foreach (['production', 'staging', 'local'] as $environment) {
            $this->app->detectEnvironment(fn (): string => $environment);
            self::assertNull(PublicProjectionSettings::current());
            $this->assertEmptyFailure($this->publish('guild', $this->snapshot(PublicProjectionFamily::Guild)), 503);
            $this->assertEmptyFailure($this->publish('watermark', $this->watermark(PublicProjectionFamily::Guild)), 503);
        }
        $this->app->detectEnvironment(fn (): string => 'preproduction');
        self::assertNotNull(PublicProjectionSettings::current());
        $this->app->detectEnvironment(fn (): string => 'testing');
        self::assertSame(0, DB::table('game_public_projection_snapshots')->count());
    }

    public function test_malformed_publications_are_refused_with_empty_400_and_store_nothing(): void
    {
        $hs = self::CONTRACT_FIXTURES['highscores'];
        $refused = [
            'unknown member' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['extra' => '1'])],
            'null member' => ['highscores', str_replace('"filter":0', '"filter":null', $hs)],
            'duplicate member' => ['highscores', str_replace('"filter":0,', '"filter":0,"filter":0,', $hs)],
            'string escape' => ['highscores', str_replace('"Aldric"', '"Ald'.chr(92).'u0072ic"', $hs)],
            'whitespace' => ['highscores', str_replace('"filter":0', '"filter": 0', $hs)],
            'newline' => ['highscores', $hs."\n"],
            'category out of range' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['category' => 13])],
            'integer as string' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['filter' => '0'])],
            'zero revision' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['projection_revision' => '0'])],
            'uint64 overflow' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['projection_revision' => '18446744073709551616'])],
            'contract version' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['contract_version' => 2])],
            'other family operation' => ['highscores', $this->snapshot(PublicProjectionFamily::CharacterProfile)],
            'uppercase id' => ['character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile, ['character_id' => strtoupper(self::CHARACTER)])],
            'non-v7 id' => ['character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile, ['character_id' => '01934f10-7c04-4001-805b-3b1122334401'])],
            'name with quote' => ['world-online', $this->snapshot(PublicProjectionFamily::WorldOnline, [], [['name' => 'Al"dric', 'vocation' => 4, 'level' => '412']])],
            'name not NFC' => ['world-online', $this->snapshot(PublicProjectionFamily::WorldOnline, [], [['name' => "Ald\u{0065}\u{0301}", 'vocation' => 4, 'level' => '412']])],
            'online not sorted' => ['world-online', $this->snapshot(PublicProjectionFamily::WorldOnline, [], [['name' => 'Beren', 'vocation' => 1, 'level' => '2'], ['name' => 'Aldric', 'vocation' => 4, 'level' => '412']])],
            'online channel member' => ['world-online', $this->snapshot(PublicProjectionFamily::WorldOnline, [], [['name' => 'Aldric', 'vocation' => 4, 'level' => '412', 'channel_id' => self::WORLD]])],
            'profile twice' => ['character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile, [], [$this->profileEntry(), $this->profileEntry()])],
            'profile account id' => ['character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile, [], [$this->profileEntry() + ['account_id' => self::WORLD]])],
            'rank not competition' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, [], [$this->row(0, '1'), $this->row(1, '3')])],
            'character listed twice' => ['highscores', $this->snapshot(PublicProjectionFamily::Highscores, [], [$this->row(0, '1'), $this->row(0, '2')])],
            'deaths not newest first' => ['character-deaths', $this->snapshot(PublicProjectionFamily::CharacterDeaths, [], [$this->death('1789000000'), $this->death('1789990000')])],
            'guild ranks too few' => ['guild', $this->snapshot(PublicProjectionFamily::Guild, [], [$this->guildEntry(2, [])])],
            'guild member unknown rank' => ['guild', $this->snapshot(PublicProjectionFamily::Guild, [], [$this->guildEntry(3, [['name' => 'Aldric', 'rank_level' => 4, 'joined_at' => '1789000000']])])],
            'guild members not ordered' => ['guild', $this->snapshot(PublicProjectionFamily::Guild, [], [$this->guildEntry(3, [
                ['name' => 'Beren', 'rank_level' => 1, 'joined_at' => '1789000000'],
                ['name' => 'Aldric', 'rank_level' => 1, 'joined_at' => '1789000000'],
            ])])],
            'guild disbanded state' => ['guild', $this->snapshot(PublicProjectionFamily::Guild, [], [['state' => 'DISBANDED'] + $this->guildEntry(3, [])])],
            'future source_observed_at' => ['guild', $this->snapshot(PublicProjectionFamily::Guild, ['source_observed_at' => '1790000022'])],
            'over byte bound' => ['character-profile', str_pad('', 2049, ' ')],
        ];
        foreach ($refused as $case => [$path, $wire]) {
            $response = $this->publish($path, $wire);
            self::assertSame(400, $response->getStatusCode(), $case);
            self::assertSame('', $response->getContent(), $case);
        }
        self::assertSame(0, DB::table('game_public_projection_snapshots')->count());
        self::assertNull(DB::table('game_public_projection_state')->value('highest_epoch'));
    }

    public function test_every_list_bound_is_accepted_at_its_limit_and_refused_one_past(): void
    {
        $cases = [
            'highscores rows' => [PublicProjectionFamily::Highscores, 1000],
            'online characters' => [PublicProjectionFamily::WorldOnline, 4096],
            'deaths' => [PublicProjectionFamily::CharacterDeaths, 20],
            'killers' => [PublicProjectionFamily::CharacterDeaths, 16],
            'guild ranks' => [PublicProjectionFamily::Guild, 20],
            'guild members' => [PublicProjectionFamily::Guild, 2000],
        ];
        $revision = 100;
        foreach ($cases as $case => [$family, $limit]) {
            $revision++;
            $accepted = $this->publish($family->path(), $this->snapshot($family, ['projection_revision' => (string) $revision], $this->boundBody($case, $limit)));
            self::assertSame(200, $accepted->getStatusCode(), $case);
            $refused = $this->publish($family->path(), $this->snapshot($family, ['projection_revision' => (string) ($revision + 1000)], $this->boundBody($case, $limit + 1)));
            self::assertSame(400, $refused->getStatusCode(), $case);
        }
    }

    public function test_ordering_supersedes_lower_pairs_and_refuses_equal_pairs_with_different_content(): void
    {
        $profile = PublicProjectionFamily::CharacterProfile;
        $this->publish('character-profile', $this->snapshot($profile))->assertOk();
        $stored = $this->stored($profile, self::CHARACTER);

        $this->publish('character-profile', $this->snapshot($profile, ['projection_revision' => '8']))
            ->assertOk()
            ->assertContent('{"contract_version":1,"result":"superseded"}');

        // An equal pair with equal content is an idempotent accept that keeps the first source_observed_at.
        $this->publish('character-profile', $this->snapshot($profile, ['source_observed_at' => '1790000010']))
            ->assertOk()
            ->assertContent('{"contract_version":1,"result":"accepted"}');
        self::assertEquals($stored, $this->stored($profile, self::CHARACTER));

        // An equal pair with different content is 409 and the first stored snapshot is kept.
        $renamed = [['name' => 'Aldrik'] + $this->profileEntry()];
        $this->assertEmptyFailure($this->publish('character-profile', $this->snapshot($profile, [], $renamed)), 409);
        self::assertEquals($stored, $this->stored($profile, self::CHARACTER));

        $this->publish('character-profile', $this->snapshot($profile, ['projection_revision' => '10'], $renamed))->assertOk();
        self::assertSame('Aldrik', $this->stored($profile, self::CHARACTER)['lookup_name']);
        self::assertSame('10', $this->stored($profile, self::CHARACTER)['projection_revision']);

        // computed_at is highscores content: an equal pair with another computed_at is 409.
        $this->publish('highscores', $this->snapshot(PublicProjectionFamily::Highscores))->assertOk();
        $this->assertEmptyFailure($this->publish('highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['computed_at' => '1790000001'])), 409);
        $this->publish('highscores', $this->snapshot(PublicProjectionFamily::Highscores, ['source_observed_at' => '1790000005']))->assertOk();
    }

    public function test_higher_epoch_invalidates_every_family_until_its_new_epoch_publication(): void
    {
        $profile = PublicProjectionFamily::CharacterProfile;
        $this->publish('character-profile', $this->snapshot($profile))->assertOk();
        $this->publish('watermark', $this->watermark($profile))->assertOk();
        self::assertNotNull($this->entry($profile, self::CHARACTER));

        // An epoch raise on another family hides every family below it.
        $this->publish('house', $this->snapshot(PublicProjectionFamily::House, ['projection_epoch' => '2', 'projection_revision' => '1']))->assertOk();
        self::assertNull($this->entry($profile, self::CHARACTER));

        // Old-epoch publications are superseded and change nothing.
        $this->publish('character-profile', $this->snapshot($profile, ['projection_revision' => '50']))
            ->assertContent('{"contract_version":1,"result":"superseded"}');
        $this->publish('watermark', $this->watermark($profile, ['complete_through' => '1790000010', 'observed_at' => '1790000011']))
            ->assertContent('{"contract_version":1,"result":"superseded"}');
        self::assertSame('9', $this->stored($profile, self::CHARACTER)['projection_revision']);

        // The new-epoch watermark alone does not serve the old-epoch entry.
        $this->publish('watermark', $this->watermark($profile, ['projection_epoch' => '2']))->assertOk();
        self::assertNull($this->entry($profile, self::CHARACTER));

        // The resync snapshot restores it, even with a lower revision than the old epoch.
        $this->publish('character-profile', $this->snapshot($profile, ['projection_epoch' => '2', 'projection_revision' => '1']))->assertOk();
        self::assertNotNull($this->entry($profile, self::CHARACTER));
    }

    public function test_watermark_gates_serving_and_refuses_future_or_inconsistent_times(): void
    {
        $online = PublicProjectionFamily::WorldOnline;
        $this->publish('world-online', $this->snapshot($online))->assertOk();
        self::assertNull($this->entry($online, self::WORLD), 'no watermark yet');

        $this->publish('watermark', $this->watermark($online))->assertOk();
        self::assertNotNull($this->entry($online, self::WORLD));

        // A lower or equal complete_through in the same epoch is superseded.
        $this->publish('watermark', $this->watermark($online, ['complete_through' => '1789999999']))
            ->assertContent('{"contract_version":1,"result":"superseded"}');

        // Future-dated, inverted or unknown-family watermarks are refused.
        $this->assertEmptyFailure($this->publish('watermark', $this->watermark($online, ['complete_through' => '1790000022', 'observed_at' => '1790000022'])), 400);
        $this->assertEmptyFailure($this->publish('watermark', $this->watermark($online, ['complete_through' => '1790000010', 'observed_at' => '1790000009'])), 400);
        $this->assertEmptyFailure($this->publish('watermark', $this->watermark($online, ['family' => 'loyalty'])), 400);

        // A stopped watermark makes the family stale and unserved: S = 30 s with 1 s uncertainty.
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_029));
        self::assertTrue($this->readModel()->fresh($online, now()->getTimestamp()));
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_030));
        self::assertFalse($this->readModel()->fresh($online, now()->getTimestamp()));
        self::assertNull($this->entry($online, self::WORLD));

        // Watermarks are per family: another family's watermark does not make this one fresh.
        $this->publish('watermark', $this->watermark(PublicProjectionFamily::Guild, ['complete_through' => '1790000029', 'observed_at' => '1790000030']))->assertOk();
        self::assertFalse($this->readModel()->fresh($online, now()->getTimestamp()));
        self::assertTrue($this->readModel()->fresh(PublicProjectionFamily::Guild, now()->getTimestamp()));

        // Disabling the consumer stops serving at once.
        config(['game-projections.public.enabled' => false]);
        self::assertFalse($this->readModel()->fresh(PublicProjectionFamily::Guild, now()->getTimestamp()));
    }

    public function test_absent_subject_is_not_served_and_a_replayed_older_snapshot_cannot_restore_it(): void
    {
        $profile = PublicProjectionFamily::CharacterProfile;
        $this->publish('character-profile', $this->snapshot($profile))->assertOk();
        $this->publish('watermark', $this->watermark($profile))->assertOk();
        self::assertNotNull($this->entry($profile, self::CHARACTER));

        $this->publish('character-profile', $this->snapshot($profile, ['projection_revision' => '10'], []))->assertOk();
        self::assertNull($this->entry($profile, self::CHARACTER));
        self::assertFalse((bool) $this->stored($profile, self::CHARACTER)['present']);
        self::assertNull($this->stored($profile, self::CHARACTER)['lookup_name']);

        $this->publish('character-profile', $this->snapshot($profile))
            ->assertContent('{"contract_version":1,"result":"superseded"}');
        self::assertNull($this->entry($profile, self::CHARACTER));
    }

    public function test_publications_are_rate_limited_per_identity_and_route(): void
    {
        config(['game-projections.public.requests_per_minute' => 1]);
        $this->publish('character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile))->assertOk();
        $limited = $this->publish('character-profile', $this->snapshot(PublicProjectionFamily::CharacterProfile));
        $this->assertEmptyFailure($limited, 429);
        self::assertNotNull($limited->headers->get('Retry-After'));

        // Another family keeps its own budget.
        $this->publish('world-online', $this->snapshot(PublicProjectionFamily::WorldOnline))->assertOk();
    }

    /**
     * @param  array<string, mixed>  $envelope
     * @param  list<array<string, mixed>>|null  $body
     */
    private function snapshot(PublicProjectionFamily $family, array $envelope = [], ?array $body = null): string
    {
        $base = ['contract_version' => 1, 'operation' => $family->operation(), 'source_authority' => self::AUTHORITY];
        $observed = ['source_observed_at' => '1790000000'];
        $wire = match ($family) {
            PublicProjectionFamily::Highscores => $base + ['world_id' => self::WORLD, 'category' => 1, 'filter' => 0, 'projection_epoch' => '1', 'projection_revision' => '17'] + $observed + [
                'computed_at' => '1790000000',
                'rows' => $body ?? [$this->row(0, '1')],
            ],
            PublicProjectionFamily::CharacterProfile => $base + ['character_id' => self::CHARACTER, 'projection_epoch' => '1', 'projection_revision' => '9'] + $observed + [
                'profile' => $body ?? [$this->profileEntry()],
            ],
            PublicProjectionFamily::CharacterDeaths => $base + ['character_id' => self::CHARACTER, 'projection_epoch' => '1', 'projection_revision' => '3'] + $observed + [
                'deaths' => $body ?? [$this->death('1789990000', ['Beren', 'Mira'])],
            ],
            PublicProjectionFamily::Guild => $base + ['guild_id' => self::GUILD, 'projection_epoch' => '1', 'projection_revision' => '5'] + $observed + [
                'guild' => $body ?? [$this->guildEntry(3, [['name' => 'Aldric', 'rank_level' => 1, 'joined_at' => '1789000000']])],
            ],
            PublicProjectionFamily::House => $base + ['world_id' => self::WORLD, 'house_key' => 'oteryn:content.house.example-street-1', 'projection_epoch' => '1', 'projection_revision' => '12'] + $observed + [
                'house' => $body ?? [[
                    'name' => 'Example Street 1', 'kind' => 'private_house', 'town' => 'oteryn:content.area.city.example',
                    'size_sqm' => '24', 'beds' => '2', 'rent_gold' => '5000', 'status' => 'OWNED',
                    'owner_kind' => 'CHARACTER', 'owner_name' => 'Aldric', 'paid_until' => '1792000000',
                ]],
            ],
            PublicProjectionFamily::WorldOnline => $base + ['world_id' => self::WORLD, 'projection_epoch' => '1', 'projection_revision' => '8812'] + $observed + [
                'characters' => $body ?? [['name' => 'Aldric', 'vocation' => 4, 'level' => '412']],
            ],
        };

        return $this->encode(array_merge($wire, $envelope));
    }

    /** @param array<string, mixed> $changes */
    private function watermark(PublicProjectionFamily $family, array $changes = []): string
    {
        return $this->encode(array_merge([
            'contract_version' => 1,
            'operation' => 'PublishPublicProjectionWatermarkV1',
            'source_authority' => self::AUTHORITY,
            'family' => $family->value,
            'projection_epoch' => '1',
            'complete_through' => '1790000000',
            'observed_at' => '1790000003',
        ], $changes));
    }

    /** @return list<array<string, mixed>> */
    private function boundBody(string $case, int $count): array
    {
        $indexes = range(0, $count - 1);

        return match ($case) {
            'highscores rows' => array_map(fn (int $i): array => $this->row($i, (string) ($i + 1)), $indexes),
            'online characters' => array_map(fn (int $i): array => ['name' => sprintf('C%04d', $i), 'vocation' => 1, 'level' => '8'], $indexes),
            'deaths' => array_map(fn (int $i): array => $this->death('1789990000'), $indexes),
            'killers' => [$this->death('1789990000', array_map(fn (int $i): string => sprintf('K%02d', $i), $indexes))],
            'guild ranks' => [$this->guildEntry($count, [])],
            default => [$this->guildEntry(3, array_map(fn (int $i): array => ['name' => sprintf('M%04d', $i), 'rank_level' => 3, 'joined_at' => '1789000000'], $indexes))],
        };
    }

    /** @return array<string, mixed> */
    private function row(int $index, string $rank): array
    {
        return [
            'rank' => $rank,
            'character_id' => $index === 0 ? self::CHARACTER : sprintf('01934f10-7c04-7001-805b-%012x', $index),
            'name' => $index === 0 ? 'Aldric' : sprintf('R%04d', $index),
            'vocation' => 4,
            'level' => '412',
            'value' => $index === 0 ? '11400000000' : '0',
        ];
    }

    /** @return array<string, mixed> */
    private function profileEntry(): array
    {
        return [
            'world_id' => self::WORLD,
            'name' => 'Aldric',
            'vocation' => 4,
            'level' => '412',
            'guild' => [['guild_id' => self::GUILD, 'rank_level' => 3]],
        ];
    }

    /**
     * @param  list<string>  $killers
     * @return array<string, mixed>
     */
    private function death(string $occurredAt, array $killers = []): array
    {
        return ['occurred_at' => $occurredAt, 'level' => '413', 'pvp' => true, 'killers' => $killers];
    }

    /**
     * @param  list<array<string, mixed>>  $members
     * @return array<string, mixed>
     */
    private function guildEntry(int $rankCount, array $members): array
    {
        $names = [1 => 'Leader', 2 => 'Vice Leader', 3 => 'Member'];
        $ranks = array_map(fn (int $level): array => ['level' => $level, 'name' => $names[$level] ?? sprintf('Rank %02d', $level)], range(1, $rankCount));

        return [
            'world_id' => self::WORLD,
            'name' => 'Red Rose',
            'state' => 'ACTIVE',
            'founded_at' => '1789000000',
            'ranks' => $ranks,
            'members' => $members,
        ];
    }

    /** @param array<string, mixed> $value */
    private function encode(array $value): string
    {
        return json_encode($value, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }

    /** @return array<string, mixed>|null */
    private function entry(PublicProjectionFamily $family, string $subjectKey): ?array
    {
        return $this->readModel()->entry($family, $subjectKey, now()->getTimestamp());
    }

    private function readModel(): PublicProjectionReadModel
    {
        return app(PublicProjectionReadModel::class);
    }

    /** @return array<string, mixed> */
    private function stored(PublicProjectionFamily $family, string $subjectKey): array
    {
        $row = DB::table('game_public_projection_snapshots')
            ->where('family', $family->value)
            ->where('subject_key', $subjectKey)
            ->first();
        self::assertNotNull($row);
        /** @var array<string, mixed> $columns */
        $columns = get_object_vars($row);

        return $columns;
    }

    /**
     * @param  array<string, string>  $server
     * @return TestResponse<Response>
     */
    private function publish(string $path, string $body, string $identity = self::IDENTITY, array $server = []): TestResponse
    {
        return $this->call('POST', '/internal/v1/game-public/'.$path, [], [], [], $this->server($server + [
            'SSL_CLIENT_S_DN' => $identity,
        ]), $body);
    }

    /**
     * @param  array<string, string>  $changes
     * @return array<string, string>
     */
    private function server(array $changes = []): array
    {
        return $changes + [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::IDENTITY,
        ];
    }

    /** @param TestResponse<Response> $response */
    private function assertEmptyFailure(TestResponse $response, int $status): void
    {
        self::assertSame($status, $response->getStatusCode());
        self::assertSame('', $response->getContent());
    }
}
