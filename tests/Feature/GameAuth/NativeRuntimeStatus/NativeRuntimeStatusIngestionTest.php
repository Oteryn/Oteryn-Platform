<?php

namespace Tests\Feature\GameAuth\NativeRuntimeStatus;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReport;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRow;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Testing\TestResponse;
use InvalidArgumentException;
use LogicException;
use ReflectionMethod;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/**
 * Platform consumer of Game `oteryn-game-native-runtime-status-v1` (login contract §7.2). The wire
 * fixture is the Game encoder's exact output; see tests/Fixtures/GameAuth/native-runtime-status-v1/provenance.json.
 */
final class NativeRuntimeStatusIngestionTest extends TestCase
{
    use RefreshDatabase;

    private const PATH = '/internal/v1/game-auth/native-runtime-status';

    private const FIXTURES = 'tests/Fixtures/GameAuth/native-runtime-status-v1/';

    private const NODE = 'CN=node-a.runtime-status';

    private const NODE_B = 'CN=node-b.runtime-status';

    private const EVIDENCE = 'CN=node-a.native-evidence';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHANNEL = '01934f10-7c03-7001-805b-3b1122334401';

    private const OTHER_CHANNEL = '01934f10-7c03-7001-805b-3b1122334402';

    private const ACK = '{"contract_version":1,"result":"accepted"}';

    protected function setUp(): void
    {
        parent::setUp();

        config([
            'game-auth.native_evidence.mtls_client_identity' => self::EVIDENCE,
            'game-auth.native_runtime_status.enabled' => true,
            'game-auth.native_runtime_status.identities' => json_encode([
                self::NODE => [self::WORLD.'/'.self::CHANNEL],
                self::NODE_B => [self::WORLD.'/'.self::CHANNEL, self::WORLD.'/'.self::OTHER_CHANNEL],
            ], JSON_THROW_ON_ERROR),
            'game-auth.native_runtime_status.freshness_seconds' => '15',
            'game-auth.native_runtime_status.clock_uncertainty_seconds' => '1',
            'game-auth.native_runtime_status.requests_per_minute' => '600',
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_016));
        $this->assign(self::CHANNEL, '1', '3', self::NODE);
    }

    public function test_exact_game_wire_is_accepted_with_the_exact_acknowledgement_and_routes(): void
    {
        $response = $this->report($this->wire());

        $response->assertOk();
        self::assertSame(self::ACK, $response->getContent());
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));

        $readModel = $this->readModel();
        self::assertSame(NativeRuntimeStatusReadModel::FRESH, $readModel->evidence(self::WORLD, self::CHANNEL, $this->now()));
        $record = $readModel->routable(self::WORLD, self::CHANNEL, $this->now());
        self::assertNotNull($record);
        self::assertSame('rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e', $record->routeRevision);
        self::assertSame('3', $record->scopeOwnershipGeneration);
        self::assertSame('observation-1', $record->runtimeObservationRevision);
        self::assertSame('offer-1', $record->offerRevision);
        self::assertSame([1, 1], [$record->protocolMajor, $record->transportProfile]);
        self::assertSame(1_790_000_015, $record->observedAt);
    }

    public function test_the_largest_grammar_valid_report_fits_and_parses(): void
    {
        $max = '18446744073709551615';
        $changes = [
            'assignment_epoch' => $max,
            'scope_ownership_generation' => $max,
            'source_revision' => $max,
            'source_authority' => str_repeat('a', 128),
            'decision_identity' => str_repeat('d', 128),
            'ready' => false,
            'published_at' => (string) PHP_INT_MAX,
            'observed_at' => (string) PHP_INT_MAX,
        ];
        foreach (['route', 'runtime_observation', 'ruleset', 'content', 'map', 'world_policy', 'offer'] as $revision) {
            $changes[$revision.'_revision'] = str_repeat('r', 64);
        }
        $body = $this->body($changes);

        self::assertLessThanOrEqual(NativeRuntimeStatusReport::MAX_REQUEST_BYTES, strlen($body));
        self::assertSame($max, NativeRuntimeStatusReport::fromWire($body)->assignmentEpoch);
    }

    public function test_heartbeat_refreshes_and_the_ordering_key_only_moves_forward(): void
    {
        $this->assertResult($this->report($this->wire()), 'accepted');
        $this->assertResult($this->report($this->body(['observed_at' => '1790000016'])), 'refreshed');
        $this->assertResult($this->report($this->body(['observed_at' => '1790000010'])), 'superseded');
        $this->assertResult($this->report($this->revision('6', '1790000017')), 'superseded');
        self::assertSame(1_790_000_016, NativeRuntimeStatusRow::int($this->stored(), 'observed_at'));
        self::assertSame('7', NativeRuntimeStatusRow::string($this->stored(), 'source_revision'));

        $this->assertResult($this->report($this->revision('8', '1790000017')), 'accepted');
        self::assertSame('8', NativeRuntimeStatusRow::string($this->stored(), 'source_revision'));
        self::assertSame(1_790_000_017, NativeRuntimeStatusRow::int($this->stored(), 'observed_at'));
    }

    public function test_equal_key_with_other_content_is_a_conflict_until_a_newer_publication(): void
    {
        $this->assertResult($this->report($this->wire()), 'accepted');

        $this->assertRefused($this->report($this->body(['ready' => false])), 409);
        $readModel = $this->readModel();
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $readModel->evidence(self::WORLD, self::CHANNEL, $this->now()));
        self::assertNull($readModel->routable(self::WORLD, self::CHANNEL, $this->now()));
        $this->assertRefused($this->report($this->body(['observed_at' => '1790000016'])), 409);

        $this->assertResult($this->report($this->revision('8', '1790000016')), 'accepted');
        self::assertNotNull($readModel->routable(self::WORLD, self::CHANNEL, $this->now()));
    }

    public function test_only_runtime_status_identities_for_the_scope_are_accepted(): void
    {
        $this->assertRefused($this->report($this->wire(), self::EVIDENCE), 401);
        $this->assertRefused($this->report($this->wire(), 'CN=unknown'), 401);
        $this->assertRefused($this->report($this->wire(), self::NODE, ['SSL_CLIENT_VERIFY' => 'FAILED:self signed']), 401);
        $this->assertRefused($this->report($this->wire(), self::NODE, ['SSL_PROTOCOL' => 'TLSv1.2']), 401);
        $this->assign(self::OTHER_CHANNEL, '1', '3', self::NODE);
        $this->assertRefused($this->report($this->body(['channel_id' => self::OTHER_CHANNEL])), 401);
        self::assertFalse(DB::table('native_runtime_status_reports')->exists());

        // A runtime-status identity may never be another purpose's identity: fail closed.
        config(['game-auth.native_runtime_status.identities' => [self::EVIDENCE => [self::WORLD.'/'.self::CHANNEL]]]);
        $this->assertRefused($this->report($this->wire(), self::EVIDENCE), 503);
    }

    public function test_reports_that_do_not_match_the_latest_assignment_are_conflicts(): void
    {
        DB::table('native_scope_assignments')->delete();
        $this->assertRefused($this->report($this->wire()), 409);

        foreach ([['2', '3', self::NODE], ['1', '4', self::NODE], ['1', '2', self::NODE], ['1', '3', self::NODE_B]] as [$epoch, $generation, $identity]) {
            $this->assign(self::CHANNEL, $epoch, $generation, $identity);
            $this->assertRefused($this->report($this->wire()), 409);
        }
        self::assertFalse(DB::table('native_runtime_status_reports')->exists());

        $this->assign(self::CHANNEL, '1', '3', self::NODE);
        $this->assertResult($this->report($this->wire()), 'accepted');

        // Node replaced: the old node's report no longer routes and its reports conflict.
        $this->assign(self::CHANNEL, '1', '4', self::NODE_B);
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $this->readModel()->evidence(self::WORLD, self::CHANNEL, $this->now()));
        $this->assertRefused($this->report($this->body(['observed_at' => '1790000016'])), 409);
        $this->assertResult($this->report($this->body([
            'scope_ownership_generation' => '4',
            'decision_identity' => 'runtime-readiness:0b:4:1:true',
            'source_revision' => '1',
        ]), self::NODE_B), 'accepted');
        self::assertSame(NativeRuntimeStatusReadModel::FRESH, $this->readModel()->evidence(self::WORLD, self::CHANNEL, $this->now()));

        // Restore reset: a higher epoch anywhere invalidates every lower-epoch scope.
        $this->assign(self::OTHER_CHANNEL, '2', '1', self::NODE_B);
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $this->readModel()->evidence(self::WORLD, self::CHANNEL, $this->now()));
        $this->assertRefused($this->report($this->body([
            'scope_ownership_generation' => '4',
            'decision_identity' => 'runtime-readiness:0b:4:2:true',
            'source_revision' => '2',
        ]), self::NODE_B), 409);
    }

    public function test_stale_or_not_ready_reports_make_routing_unavailable(): void
    {
        $readModel = $this->readModel();
        self::assertSame(NativeRuntimeStatusReadModel::UNAVAILABLE, $readModel->evidence(self::WORLD, self::CHANNEL, $this->now()));
        $this->assertResult($this->report($this->wire()), 'accepted');

        // F = 15 s, uncertainty 1 s: fresh while now - observed_at + 1 <= 15.
        self::assertNotNull($readModel->routable(self::WORLD, self::CHANNEL, 1_790_000_029));
        self::assertSame(NativeRuntimeStatusReadModel::STALE, $readModel->evidence(self::WORLD, self::CHANNEL, 1_790_000_030));
        self::assertNull($readModel->routable(self::WORLD, self::CHANNEL, 1_790_000_030));

        $this->assertResult($this->report($this->body([
            'source_revision' => '8',
            'ready' => false,
            'decision_identity' => 'runtime-readiness:0a:3:8:false',
        ])), 'accepted');
        self::assertSame(NativeRuntimeStatusReadModel::FRESH, $readModel->evidence(self::WORLD, self::CHANNEL, $this->now()));
        self::assertNull($readModel->routable(self::WORLD, self::CHANNEL, $this->now()));

        config(['game-auth.native_runtime_status.enabled' => false]);
        self::assertSame(NativeRuntimeStatusReadModel::UNAVAILABLE, $readModel->evidence(self::WORLD, self::CHANNEL, $this->now()));
    }

    public function test_reports_outside_the_game_grammar_are_malformed(): void
    {
        $wire = $this->wire();
        $bodies = [
            'unknown endpoint member' => $this->body(['host' => 'play.oteryn.test']),
            'missing member' => $this->body([], ['offer_revision']),
            'null member' => $this->body(['offer_revision' => null]),
            'nested member' => $this->body(['offer_revision' => ['offer-1']]),
            'duplicate member' => str_replace('"ready":true,', '"ready":true,"ready":true,', $wire),
            'escaped duplicate member' => str_replace('"ready":true,', '"ready":true,"ready":true,', $wire),
            'trailing data' => $wire.'{}',
            'list root' => '['.$wire.']',
            'uppercase world_id' => $this->body(['world_id' => strtoupper(self::WORLD)]),
            'non-v7 channel_id' => $this->body(['channel_id' => '01934f10-7c03-4001-805b-3b1122334401']),
            'zero epoch' => $this->body(['assignment_epoch' => '0']),
            'leading zero generation' => $this->body(['scope_ownership_generation' => '03']),
            'integer revision' => $this->body(['source_revision' => 7]),
            'uint64 overflow' => $this->body(['source_revision' => '18446744073709551616']),
            'negative time' => $this->body(['published_at' => '-1']),
            'observed before published' => $this->body(['observed_at' => '1789999999']),
            'future observed_at' => $this->body(['observed_at' => '1790000018']),
            'protocol_major 2' => $this->body(['protocol_major' => 2]),
            'protocol_major string' => $this->body(['protocol_major' => '1']),
            'long route_revision' => $this->body(['route_revision' => str_repeat('r', 65)]),
            'space in revision' => $this->body(['offer_revision' => 'offer 1']),
            'quote in decision_identity' => $this->body(['decision_identity' => 'a"b']),
            'newline in revision' => $this->body(['map_revision' => "map-1\n"]),
            'string ready' => $this->body(['ready' => 'true']),
            'contract_version 2' => $this->body(['contract_version' => 2]),
            'other operation' => $this->body(['operation' => 'ReportScopeAssignmentV1']),
            'oversized body' => $wire.str_repeat(' ', NativeRuntimeStatusReport::MAX_REQUEST_BYTES - strlen($wire) + 1),
        ];
        foreach ($bodies as $case => $body) {
            $this->assertRefused($this->report($body), 400, $case);
        }
        $this->assertRefused($this->report($wire, self::NODE, ['CONTENT_TYPE' => 'text/plain']), 400);
        self::assertFalse(DB::table('native_runtime_status_reports')->exists());

        $this->expectException(InvalidArgumentException::class);
        NativeRuntimeStatusReport::fromWire('');
    }

    public function test_switch_is_default_off_and_misconfiguration_fails_closed(): void
    {
        config(['game-auth.native_runtime_status.enabled' => null]);
        $this->assertRefused($this->report($this->wire()), 503);

        config(['game-auth.native_runtime_status.enabled' => true, 'game-auth.native_runtime_status.identities' => '{"CN=a":["not-a-scope"]}']);
        $this->assertRefused($this->report($this->wire()), 503);

        config(['game-auth.native_runtime_status.identities' => [self::NODE => [self::WORLD.'/'.self::CHANNEL]], 'game-auth.native_runtime_status.freshness_seconds' => '0']);
        $this->assertRefused($this->report($this->wire()), 503);
        self::assertFalse(DB::table('native_runtime_status_reports')->exists());

        $default = require base_path('config/game-auth.php');
        self::assertIsArray($default);
        self::assertIsArray($default['native_runtime_status']);
        self::assertFalse($default['native_runtime_status']['enabled']);
    }

    public function test_rate_limit_is_an_empty_429_per_identity(): void
    {
        config(['game-auth.native_runtime_status.requests_per_minute' => 1]);

        $this->assertResult($this->report($this->wire()), 'accepted');
        $limited = $this->report($this->body(['observed_at' => '1790000016']));
        $this->assertRefused($limited, 429);
        self::assertNotNull($limited->headers->get('Retry-After'));
    }

    public function test_migration_rollback_is_refused_while_rows_exist(): void
    {
        $this->assertResult($this->report($this->wire()), 'accepted');
        $migration = require database_path('migrations/2026_09_30_120000_add_native_runtime_status_read_model.php');
        self::assertIsObject($migration);

        try {
            (new ReflectionMethod($migration, 'down'))->invoke($migration);
            self::fail('Rollback must be refused while runtime status rows exist.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('rollback refused', $exception->getMessage());
        }
        self::assertTrue(DB::table('native_runtime_status_reports')->exists());
        self::assertTrue(DB::table('native_scope_assignments')->exists());
    }

    private function wire(): string
    {
        $raw = file_get_contents(base_path(self::FIXTURES.'report.json'));
        self::assertIsString($raw);
        self::assertStringEndsWith("\n", $raw);
        $wire = substr($raw, 0, -1);
        $provenance = json_decode((string) file_get_contents(base_path(self::FIXTURES.'provenance.json')), true, 8, JSON_THROW_ON_ERROR);
        self::assertIsArray($provenance);
        self::assertIsArray($provenance['files']);
        self::assertIsArray($provenance['files']['report.json']);
        self::assertSame(hash('sha256', $wire), $provenance['files']['report.json']['wire_sha256']);

        return $wire;
    }

    /**
     * @param  array<string, mixed>  $changes
     * @param  list<string>  $remove
     */
    private function body(array $changes, array $remove = []): string
    {
        $decoded = json_decode($this->wire(), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($decoded);
        foreach ($remove as $name) {
            unset($decoded[$name]);
        }

        return json_encode(array_merge($decoded, $changes), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    private function revision(string $sourceRevision, string $observedAt): string
    {
        return $this->body([
            'source_revision' => $sourceRevision,
            'decision_identity' => 'runtime-readiness:0a:3:'.$sourceRevision.':true',
            'observed_at' => $observedAt,
        ]);
    }

    /**
     * @param  array<string, string>  $server
     * @return TestResponse<Response>
     */
    private function report(string $body, string $identity = self::NODE, array $server = []): TestResponse
    {
        return $this->call('POST', self::PATH, [], [], [], $server + [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => $identity,
        ], $body);
    }

    /** @param TestResponse<Response> $response */
    private function assertResult(TestResponse $response, string $result): void
    {
        $response->assertOk();
        self::assertSame('{"contract_version":1,"result":"'.$result.'"}', $response->getContent());
    }

    /** @param TestResponse<Response> $response */
    private function assertRefused(TestResponse $response, int $status, string $case = ''): void
    {
        self::assertSame($status, $response->getStatusCode(), $case);
        self::assertSame('', $response->getContent(), $case);
    }

    private function assign(string $channelId, string $epoch, string $generation, string $identity): void
    {
        DB::table('native_scope_assignments')->updateOrInsert(
            ['world_id' => self::WORLD, 'channel_id' => $channelId],
            ['assignment_epoch' => $epoch, 'ownership_generation' => $generation, 'node_identity' => $identity, 'assigned_at' => 1_789_999_990],
        );
    }

    private function stored(): object
    {
        $row = DB::table('native_runtime_status_reports')->where('channel_id', self::CHANNEL)->first();
        self::assertNotNull($row);

        return $row;
    }

    private function readModel(): NativeRuntimeStatusReadModel
    {
        return app(NativeRuntimeStatusReadModel::class);
    }

    private function now(): int
    {
        return CarbonImmutable::now()->getTimestamp();
    }
}
