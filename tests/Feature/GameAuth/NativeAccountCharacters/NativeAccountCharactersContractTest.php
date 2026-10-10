<?php

namespace Tests\Feature\GameAuth\NativeAccountCharacters;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersAccountView;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Log\Events\MessageLogged;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Event;
use Illuminate\Testing\TestResponse;
use InvalidArgumentException;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/**
 * LCFA projection §9 / login contract §16 projection rows that the ingestion and HTTP tests do not
 * already cover: Game wire fixtures, exact schema edges, names, epoch ordering and log privacy.
 */
final class NativeAccountCharactersContractTest extends TestCase
{
    use RefreshDatabase;

    private const IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const OTHER_ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c7';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const SNAPSHOT_PATH = '/internal/v1/game-auth/native-account-characters';

    private const WATERMARK_PATH = '/internal/v1/game-auth/native-account-characters/watermark';

    /**
     * Byte-equal to the Game producer's encoder fixtures at Game commit
     * ee71e79eccd1d498d6c39ea25ac01ee74ccd118c,
     * apps/game-server/src/native_admission_source/account_characters_tests.rs.
     */
    private const GAME_SNAPSHOT = '{"contract_version":1,"operation":"PublishAccountCharactersV1","source_authority":"oteryn:character-authority:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","projection_revision":"42","source_observed_at":"1790000000","characters":[{"character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","availability":"AVAILABLE"}]}';

    private const GAME_WATERMARK = '{"contract_version":1,"operation":"PublishProjectionWatermarkV1","source_authority":"oteryn:character-authority:primary","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}';

    private const GAME_ACCEPTED = '{"contract_version":1,"result":"accepted"}';

    private const GAME_SUPERSEDED = '{"contract_version":1,"result":"superseded"}';

    protected function setUp(): void
    {
        parent::setUp();
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::IDENTITY],
            'game-auth.native_account_characters.source_authority' => self::AUTHORITY,
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 600,
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_005));
    }

    public function test_game_encoder_fixtures_are_accepted_byte_for_byte_with_the_exact_acknowledgements(): void
    {
        $snapshot = $this->publish(self::SNAPSHOT_PATH, self::GAME_SNAPSHOT);
        $snapshot->assertOk();
        self::assertSame(self::GAME_ACCEPTED, $snapshot->getContent());

        $watermark = $this->publish(self::WATERMARK_PATH, self::GAME_WATERMARK);
        $watermark->assertOk();
        self::assertSame(self::GAME_ACCEPTED, $watermark->getContent());

        // The same watermark again does not advance complete_through.
        self::assertSame(self::GAME_SUPERSEDED, $this->publish(self::WATERMARK_PATH, self::GAME_WATERMARK)->getContent());

        $view = $this->readModel()->viewForAccount(self::ACCOUNT, $this->now());
        self::assertSame(NativeAccountCharactersAccountView::READY, $view->state);
        self::assertSame('Aldric', $view->characters[0]->name);
    }

    public function test_exact_schema_refuses_every_malformed_envelope_and_entry(): void
    {
        $entry = '{"character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","availability":"AVAILABLE"}';
        $snapshots = [
            'unknown member' => str_replace('"characters":', '"extra":1,"characters":', self::GAME_SNAPSHOT),
            'duplicate member' => str_replace('"characters":', '"projection_revision":"42","characters":', self::GAME_SNAPSHOT),
            'null member' => str_replace('"projection_revision":"42"', '"projection_revision":null', self::GAME_SNAPSHOT),
            'uppercase world id' => str_replace('"world_id":"01934f10-7c02-7001-805b-3b1122334401"', '"world_id":"01934F10-7C02-7001-805B-3B1122334401"', self::GAME_SNAPSHOT),
            'UUIDv4 account id' => str_replace(self::ACCOUNT, '0190f2a1-3b4c-4d5e-8f60-718293a4b5c6', self::GAME_SNAPSHOT),
            'zero revision' => str_replace('"projection_revision":"42"', '"projection_revision":"0"', self::GAME_SNAPSHOT),
            'numeric revision' => str_replace('"projection_revision":"42"', '"projection_revision":42', self::GAME_SNAPSHOT),
            'uint64 overflow' => str_replace('"projection_epoch":"1"', '"projection_epoch":"18446744073709551616"', self::GAME_SNAPSHOT),
            'wrong version' => str_replace('"contract_version":1', '"contract_version":2', self::GAME_SNAPSHOT),
            'wrong operation' => str_replace('PublishAccountCharactersV1', 'PublishAccountCharactersV2', self::GAME_SNAPSHOT),
            'duplicate entry' => str_replace('"characters":['.$entry.']', '"characters":['.$entry.','.$entry.']', self::GAME_SNAPSHOT),
            'entry unknown member' => str_replace('"availability":"AVAILABLE"}', '"availability":"AVAILABLE","level":1}', self::GAME_SNAPSHOT),
            'unknown availability' => str_replace('"availability":"AVAILABLE"', '"availability":"ONLINE"', self::GAME_SNAPSHOT),
            '65 entries' => $this->snapshotWithEntries(65, 'N'),
            'oversize' => str_replace('"name":"Aldric"', '"name":"'.str_repeat('N', 64).'"', self::GAME_SNAPSHOT).str_repeat(' ', NativeAccountCharactersWire::SNAPSHOT_MAX_BYTES),
            'trailing newline' => self::GAME_SNAPSHOT."\n",
        ];
        foreach ($snapshots as $case => $wire) {
            $this->assertRefusedWire(fn (): mixed => NativeAccountCharactersSnapshot::fromWire($wire), $case);
        }

        $watermarks = [
            'unknown member' => str_replace('"observed_at"', '"extra":1,"observed_at"', self::GAME_WATERMARK),
            'null epoch' => str_replace('"projection_epoch":"1"', '"projection_epoch":null', self::GAME_WATERMARK),
            'wrong version' => str_replace('"contract_version":1', '"contract_version":0', self::GAME_WATERMARK),
            'complete after observed' => str_replace('"complete_through":"1790000000"', '"complete_through":"1790000004"', self::GAME_WATERMARK),
            'oversize' => self::GAME_WATERMARK.str_repeat(' ', NativeAccountCharactersWire::WATERMARK_MAX_BYTES),
        ];
        foreach ($watermarks as $case => $wire) {
            $this->assertRefusedWire(fn (): mixed => NativeAccountCharactersWatermark::fromWire($wire), 'watermark '.$case);
        }

        $this->assertEmptyFailure($this->publish(self::SNAPSHOT_PATH, $snapshots['65 entries']), 400);
        $this->assertEmptyFailure($this->publish(self::SNAPSHOT_PATH, $snapshots['duplicate member']), 400);
        self::assertSame(0, DB::table('native_account_character_snapshots')->count());
    }

    public function test_names_refused_by_the_game_are_refused_and_the_largest_snapshot_fits(): void
    {
        foreach (['', str_repeat('N', 65), 'Al"dric', 'Al\\dric', "Al\ndric", "Al\u{7f}dric", "Al\u{202e}dric", "Al\u{2066}dric", "Al\u{200b}dric", "Al\u{feff}dric", "Alde\u{301}ric"] as $name) {
            $this->assertRefusedWire(
                fn (): mixed => NativeAccountCharactersWire::characterName($name),
                'name '.bin2hex($name),
            );
        }
        self::assertSame('Al Dric', NativeAccountCharactersWire::characterName('Al Dric'));
        self::assertSame('Þórr', NativeAccountCharactersWire::characterName('Þórr'));

        $largest = $this->snapshotWithEntries(64, str_repeat('N', 64), 'UNAVAILABLE');
        self::assertLessThanOrEqual(NativeAccountCharactersWire::SNAPSHOT_MAX_BYTES, strlen($largest));
        $response = $this->publish(self::SNAPSHOT_PATH, $largest);
        $response->assertOk();
        self::assertSame(self::GAME_ACCEPTED, $response->getContent());
        self::assertSame(64, DB::table('native_account_character_rows')->where('account_id', self::ACCOUNT)->count());
    }

    public function test_identical_equal_pair_is_idempotent_and_keeps_the_first_source_observed_at(): void
    {
        $results = [
            $this->snapshot(self::GAME_SNAPSHOT),
            $this->snapshot(self::GAME_SNAPSHOT),
            $this->snapshot(str_replace('"source_observed_at":"1790000000"', '"source_observed_at":"1790000004"', self::GAME_SNAPSHOT)),
        ];
        self::assertSame(['accepted', 'accepted', 'accepted'], $results);
        self::assertSame(1, DB::table('native_account_character_snapshots')
            ->where('source_observed_at', 1_790_000_000)
            ->where('invalid', false)
            ->count());

        self::assertSame('superseded', $this->snapshot(str_replace('"projection_revision":"42"', '"projection_revision":"41"', self::GAME_SNAPSHOT)));
    }

    public function test_lower_epoch_publications_are_superseded_and_never_make_the_feed_live(): void
    {
        self::assertSame('accepted', $this->snapshot($this->epoch(self::GAME_SNAPSHOT, '2')));
        self::assertSame(NativeAccountCharactersReadModel::STALE, $this->readModel()->feedEvidence($this->now()));

        self::assertSame('superseded', $this->watermark(self::GAME_WATERMARK));
        self::assertSame('superseded', $this->snapshot($this->account(self::GAME_SNAPSHOT, self::OTHER_ACCOUNT)));
        self::assertSame(NativeAccountCharactersReadModel::STALE, $this->readModel()->feedEvidence($this->now()));
        self::assertSame(0, DB::table('native_account_character_snapshots')->where('account_id', self::OTHER_ACCOUNT)->count());

        self::assertSame('accepted', $this->watermark($this->epoch(self::GAME_WATERMARK, '2')));
        self::assertSame(NativeAccountCharactersReadModel::LIVE, $this->readModel()->feedEvidence($this->now()));

        // An equal-epoch watermark that advances complete_through keeps the feed live.
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_025));
        self::assertSame('accepted', $this->watermark(str_replace(
            ['"complete_through":"1790000000"', '"observed_at":"1790000003"'],
            ['"complete_through":"1790000020"', '"observed_at":"1790000024"'],
            $this->epoch(self::GAME_WATERMARK, '2'),
        )));
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_040));
        self::assertSame(NativeAccountCharactersReadModel::LIVE, $this->readModel()->feedEvidence($this->now()));
        self::assertSame(NativeAccountCharactersAccountView::READY, $this->readModel()->viewForAccount(self::ACCOUNT, $this->now())->state);
    }

    public function test_an_epoch_raise_by_snapshot_invalidates_every_lower_account_and_stales_the_feed(): void
    {
        $this->snapshot(self::GAME_SNAPSHOT);
        $this->snapshot($this->account(self::GAME_SNAPSHOT, self::OTHER_ACCOUNT));
        $this->watermark(self::GAME_WATERMARK);
        self::assertSame(NativeAccountCharactersAccountView::READY, $this->readModel()->viewForAccount(self::OTHER_ACCOUNT, $this->now())->state);

        self::assertSame('accepted', $this->snapshot($this->epoch(self::GAME_SNAPSHOT, '2')));
        self::assertSame(NativeAccountCharactersReadModel::STALE, $this->readModel()->feedEvidence($this->now()));

        $this->watermark($this->epoch(self::GAME_WATERMARK, '2'));
        self::assertSame(NativeAccountCharactersAccountView::READY, $this->readModel()->viewForAccount(self::ACCOUNT, $this->now())->state);
        self::assertSame(NativeAccountCharactersAccountView::INVALID, $this->readModel()->viewForAccount(self::OTHER_ACCOUNT, $this->now())->state);
    }

    public function test_unix_millisecond_epochs_above_two_to_the_thirty_two_compare_numerically(): void
    {
        self::assertSame(-1, NativeAccountCharactersWire::compareUint64('9999999999999', '10000000000000'));
        self::assertSame(1, NativeAccountCharactersWire::compareUint64('1790000000001', '1790000000000'));
        self::assertSame(1, NativeAccountCharactersWire::compareUint64('18446744073709551615', '4294967296'));

        $this->snapshot($this->epoch(self::GAME_SNAPSHOT, '9999999999999'));
        self::assertSame('accepted', $this->watermark($this->epoch(self::GAME_WATERMARK, '9999999999999')));
        self::assertSame('accepted', $this->snapshot($this->epoch(self::GAME_SNAPSHOT, '10000000000000')));
        self::assertSame('10000000000000', DB::table('native_account_character_projection_state')->value('highest_epoch'));
        self::assertSame('superseded', $this->watermark($this->epoch(self::GAME_WATERMARK, '9999999999999')));
        self::assertSame('accepted', $this->watermark($this->epoch(self::GAME_WATERMARK, '10000000000000')));
        self::assertSame(NativeAccountCharactersAccountView::READY, $this->readModel()->viewForAccount(self::ACCOUNT, $this->now())->state);
    }

    public function test_publications_and_refusals_never_log_secrets_account_ids_or_names(): void
    {
        $logged = [];
        Event::listen(MessageLogged::class, function (MessageLogged $event) use (&$logged): void {
            $logged[] = $event->message.' '.json_encode($event->context);
        });

        $this->publish(self::SNAPSHOT_PATH, self::GAME_SNAPSHOT)->assertOk();
        $this->publish(self::WATERMARK_PATH, self::GAME_WATERMARK)->assertOk();
        $this->assertEmptyFailure($this->publish(self::SNAPSHOT_PATH, str_replace('"name":"Aldric"', '"name":"Aldric Prime"', self::GAME_SNAPSHOT)), 409);
        $this->assertEmptyFailure($this->publish(self::SNAPSHOT_PATH, str_replace('"name":"Aldric"', '"name":"Al\\\\dric"', self::GAME_SNAPSHOT)), 400);
        $this->assertEmptyFailure($this->publish(self::SNAPSHOT_PATH, self::GAME_SNAPSHOT, 'CN=other'), 401);

        $all = implode("\n", $logged);
        foreach ([self::ACCOUNT, 'Aldric', self::IDENTITY, 'CN=other'] as $private) {
            self::assertStringNotContainsString($private, $all);
        }
    }

    private function snapshotWithEntries(int $count, string $name, string $availability = 'AVAILABLE'): string
    {
        $characters = [];
        for ($index = 1; $index <= $count; $index++) {
            $characters[] = [
                'character_id' => sprintf('01934f10-7c04-7001-805b-3b11223344%02x', $index),
                'world_id' => self::WORLD,
                'name' => $name,
                'availability' => $availability,
            ];
        }

        return json_encode([
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::AUTHORITY,
            'account_id' => self::ACCOUNT,
            'projection_epoch' => '1',
            'projection_revision' => '42',
            'source_observed_at' => '1790000000',
            'characters' => $characters,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }

    private function epoch(string $wire, string $epoch): string
    {
        return str_replace('"projection_epoch":"1"', '"projection_epoch":"'.$epoch.'"', $wire);
    }

    private function account(string $wire, string $accountId): string
    {
        return str_replace(self::ACCOUNT, $accountId, $wire);
    }

    private function snapshot(string $wire): string
    {
        return $this->ingestion()->snapshot($this->settings(), self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($wire), $this->now());
    }

    private function watermark(string $wire): string
    {
        return $this->ingestion()->watermark($this->settings(), self::IDENTITY, NativeAccountCharactersWatermark::fromWire($wire), $this->now());
    }

    private function assertRefusedWire(callable $parse, string $case): void
    {
        try {
            $parse();
            self::fail('LCFA wire case must be refused: '.$case);
        } catch (InvalidArgumentException) {
            $this->addToAssertionCount(1);
        }
    }

    /** @return TestResponse<Response> */
    private function publish(string $path, string $body, string $identity = self::IDENTITY): TestResponse
    {
        return $this->call('POST', $path, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => $identity,
        ], $body);
    }

    /** @param TestResponse<Response> $response */
    private function assertEmptyFailure(TestResponse $response, int $status): void
    {
        self::assertSame($status, $response->getStatusCode());
        self::assertSame('', $response->getContent());
    }

    private function ingestion(): NativeAccountCharactersIngestion
    {
        return app(NativeAccountCharactersIngestion::class);
    }

    private function settings(): NativeAccountCharactersSettings
    {
        return NativeAccountCharactersSettings::current()
            ?? throw new \RuntimeException('LCFA settings must be valid in the contract test.');
    }

    private function readModel(): NativeAccountCharactersReadModel
    {
        return app(NativeAccountCharactersReadModel::class);
    }

    private function now(): int
    {
        return CarbonImmutable::now()->getTimestamp();
    }
}
