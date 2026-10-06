<?php

namespace Tests\Feature\GameAuth\NativeAccountCharacters;

use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersIngestion;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersReadModel;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersRefused;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSettings;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersSnapshot;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWatermark;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use Tests\TestCase;

final class NativeAccountCharactersIngestionTest extends TestCase
{
    use RefreshDatabase;

    private const IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    protected function setUp(): void
    {
        parent::setUp();

        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.identities' => [self::IDENTITY],
            'game-auth.native_account_characters.source_authority' => self::AUTHORITY,
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_020));
    }

    public function test_snapshot_and_watermark_make_an_authoritative_private_read_model(): void
    {
        self::assertSame('accepted', $this->ingestion()->snapshot(
            $this->settings(),
            self::IDENTITY,
            NativeAccountCharactersSnapshot::fromWire($this->snapshotWire()),
            $this->now(),
        ));
        self::assertSame(NativeAccountCharactersReadModel::STALE, $this->readModel()->feedEvidence($this->now()));
        self::assertNull($this->readModel()->forAccount(self::ACCOUNT, $this->now()));

        self::assertSame('accepted', $this->ingestion()->watermark(
            $this->settings(),
            self::IDENTITY,
            NativeAccountCharactersWatermark::fromWire($this->watermarkWire()),
            $this->now(),
        ));

        self::assertSame(NativeAccountCharactersReadModel::LIVE, $this->readModel()->feedEvidence($this->now()));
        $characters = $this->readModel()->forAccount(self::ACCOUNT, $this->now());
        self::assertNotNull($characters);
        self::assertCount(1, $characters);
        self::assertSame(self::CHARACTER, $characters[0]->characterId);
        self::assertSame(self::WORLD, $characters[0]->worldId);
        self::assertSame('Aldric', $characters[0]->name);
        self::assertSame('AVAILABLE', $characters[0]->availability);
    }

    public function test_equal_key_conflict_persists_invalid_state_until_a_higher_revision(): void
    {
        $ingestion = $this->ingestion();
        $settings = $this->settings();
        self::assertSame('accepted', $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire()), $this->now()));
        self::assertSame('accepted', $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire([
            'source_observed_at' => '1790000001',
        ])), $this->now()));

        try {
            $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire([], [
                ['character_id' => self::CHARACTER, 'world_id' => self::WORLD, 'name' => 'Aldric Prime', 'availability' => 'AVAILABLE'],
            ])), $this->now());
            self::fail('Equal key with different content must conflict.');
        } catch (NativeAccountCharactersRefused $refused) {
            self::assertSame(409, $refused->status);
        }
        self::assertSame(1, DB::table('native_account_character_snapshots')->where('account_id', self::ACCOUNT)->value('invalid'));

        self::assertSame('accepted', $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire([
            'projection_revision' => '43',
            'source_observed_at' => '1790000002',
        ], [
            ['character_id' => self::CHARACTER, 'world_id' => self::WORLD, 'name' => 'Aldric Prime', 'availability' => 'AVAILABLE'],
        ])), $this->now()));
        self::assertSame(0, DB::table('native_account_character_snapshots')->where('account_id', self::ACCOUNT)->value('invalid'));
    }

    public function test_higher_epoch_from_watermark_invalidates_every_old_account_until_resync(): void
    {
        $ingestion = $this->ingestion();
        $settings = $this->settings();
        $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire()), $this->now());
        $ingestion->watermark($settings, self::IDENTITY, NativeAccountCharactersWatermark::fromWire($this->watermarkWire()), $this->now());
        self::assertNotNull($this->readModel()->forAccount(self::ACCOUNT, $this->now()));

        self::assertSame('accepted', $ingestion->watermark($settings, self::IDENTITY, NativeAccountCharactersWatermark::fromWire($this->watermarkWire([
            'projection_epoch' => '2',
            'complete_through' => '1790000015',
            'observed_at' => '1790000018',
        ])), $this->now()));
        self::assertNull($this->readModel()->forAccount(self::ACCOUNT, $this->now()));

        self::assertSame('superseded', $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire([
            'projection_epoch' => '1',
            'projection_revision' => '43',
        ])), $this->now()));

        self::assertSame('accepted', $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire([
            'projection_epoch' => '2',
            'projection_revision' => '1',
            'source_observed_at' => '1790000017',
        ])), $this->now()));
        self::assertNotNull($this->readModel()->forAccount(self::ACCOUNT, $this->now()));
    }

    public function test_watermark_freshness_fails_closed_and_never_uses_observed_at_as_liveness(): void
    {
        $ingestion = $this->ingestion();
        $settings = $this->settings();
        $ingestion->snapshot($settings, self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire()), $this->now());
        $ingestion->watermark($settings, self::IDENTITY, NativeAccountCharactersWatermark::fromWire($this->watermarkWire([
            'complete_through' => '1789999989',
            'observed_at' => '1790000019',
        ])), $this->now());

        self::assertSame(NativeAccountCharactersReadModel::STALE, $this->readModel()->feedEvidence($this->now()));
        self::assertNull($this->readModel()->forAccount(self::ACCOUNT, $this->now()));

        // A later Platform clock rollback must not turn a stored future watermark into live evidence.
        self::assertSame(NativeAccountCharactersReadModel::STALE, $this->readModel()->feedEvidence(1_790_000_008));
        self::assertNull($this->readModel()->forAccount(self::ACCOUNT, 1_790_000_008));
    }

    public function test_strict_wire_rejects_unsorted_characters_invalid_names_and_noncanonical_json(): void
    {
        $other = '01934f10-7c04-7001-805b-3b1122334402';
        foreach ([
            $this->snapshotWire([], [
                ['character_id' => $other, 'world_id' => self::WORLD, 'name' => 'B', 'availability' => 'AVAILABLE'],
                ['character_id' => self::CHARACTER, 'world_id' => self::WORLD, 'name' => 'A', 'availability' => 'AVAILABLE'],
            ]),
            $this->snapshotWire([], [
                ['character_id' => self::CHARACTER, 'world_id' => self::WORLD, 'name' => "A\u{200D}B", 'availability' => 'AVAILABLE'],
            ]),
            str_replace('{"contract_version"', '{ "contract_version"', $this->snapshotWire()),
        ] as $wire) {
            try {
                NativeAccountCharactersSnapshot::fromWire($wire);
                self::fail('Invalid LCFA wire must be rejected.');
            } catch (InvalidArgumentException) {
                continue;
            }
        }
    }

    public function test_wrong_identity_authority_and_future_source_time_fail_closed(): void
    {
        $snapshot = NativeAccountCharactersSnapshot::fromWire($this->snapshotWire());
        foreach ([
            ['CN=other', $snapshot, 401],
            [self::IDENTITY, NativeAccountCharactersSnapshot::fromWire($this->snapshotWire(['source_observed_at' => '1790000022'])), 400],
        ] as [$identity, $report, $status]) {
            try {
                $this->ingestion()->snapshot($this->settings(), $identity, $report, $this->now());
                self::fail('Publication must be refused.');
            } catch (NativeAccountCharactersRefused $refused) {
                self::assertSame($status, $refused->status);
            }
        }

        config(['game-auth.native_account_characters.source_authority' => 'other-authority']);
        $this->expectException(NativeAccountCharactersRefused::class);
        $this->ingestion()->snapshot($this->settings(), self::IDENTITY, $snapshot, $this->now());
    }

    /**
     * @param  array<string, mixed>  $changes
     * @param  list<array<string, string>>|null  $characters
     */
    private function snapshotWire(array $changes = [], ?array $characters = null): string
    {
        $body = [
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::AUTHORITY,
            'account_id' => self::ACCOUNT,
            'projection_epoch' => '1',
            'projection_revision' => '42',
            'source_observed_at' => '1790000000',
            'characters' => $characters ?? [
                ['character_id' => self::CHARACTER, 'world_id' => self::WORLD, 'name' => 'Aldric', 'availability' => 'AVAILABLE'],
            ],
        ];

        return json_encode(array_merge($body, $changes), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }

    /** @param array<string, mixed> $changes */
    private function watermarkWire(array $changes = []): string
    {
        return json_encode(array_merge([
            'contract_version' => 1,
            'operation' => 'PublishProjectionWatermarkV1',
            'source_authority' => self::AUTHORITY,
            'projection_epoch' => '1',
            'complete_through' => '1790000010',
            'observed_at' => '1790000018',
        ], $changes), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    private function ingestion(): NativeAccountCharactersIngestion
    {
        return app(NativeAccountCharactersIngestion::class);
    }

    private function settings(): NativeAccountCharactersSettings
    {
        return NativeAccountCharactersSettings::current()
            ?? throw new \RuntimeException('LCFA settings must be valid in the test.');
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
