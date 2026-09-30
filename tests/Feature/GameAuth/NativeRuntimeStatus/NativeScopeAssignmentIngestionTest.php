<?php

namespace Tests\Feature\GameAuth\NativeRuntimeStatus;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusRow;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Testing\TestResponse;
use LogicException;
use ReflectionMethod;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/**
 * Platform consumer of Game `ReportScopeAssignmentV1` (`oteryn-game-native-runtime-status-v1` §5, §6 at
 * Game main@07cd8bcc; login contract §7.2). The wire is the §5 example with a configured node identity.
 */
final class NativeScopeAssignmentIngestionTest extends TestCase
{
    use RefreshDatabase;

    private const ASSIGN = '/internal/v1/game-auth/native-scope-assignments';

    private const RUNTIME = '/internal/v1/game-auth/native-runtime-status';

    private const OPS = 'CN=oteryn-game-ops.ownership-authority';

    private const NODE = 'CN=node-a.runtime-status';

    private const NODE_B = 'CN=node-b.runtime-status';

    private const EVIDENCE = 'CN=node-a.native-evidence';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHANNEL = '01934f10-7c03-7001-805b-3b1122334401';

    private const OTHER_CHANNEL = '01934f10-7c03-7001-805b-3b1122334402';

    protected function setUp(): void
    {
        parent::setUp();

        $scope = self::WORLD.'/'.self::CHANNEL;
        $other = self::WORLD.'/'.self::OTHER_CHANNEL;
        config([
            'game-auth.native_evidence.mtls_client_identity' => self::EVIDENCE,
            'game-auth.native_runtime_status.enabled' => true,
            'game-auth.native_runtime_status.identities' => [self::NODE => [$scope], self::NODE_B => [$scope, $other]],
            'game-auth.native_runtime_status.freshness_seconds' => '15',
            'game-auth.native_runtime_status.clock_uncertainty_seconds' => '1',
            'game-auth.native_runtime_status.requests_per_minute' => '600',
            'game-auth.native_scope_assignment.enabled' => true,
            'game-auth.native_scope_assignment.identities' => json_encode([self::OPS => [$scope, $other]], JSON_THROW_ON_ERROR),
            'game-auth.native_scope_assignment.requests_per_minute' => '60',
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_016));
    }

    public function test_exact_game_wire_is_accepted_and_makes_the_matching_runtime_report_routable(): void
    {
        $runtime = $this->runtimeWire();
        $this->assertRefused($this->send(self::RUNTIME, $runtime, self::NODE), 409);

        $response = $this->send(self::ASSIGN, $this->wire());
        $this->assertResult($response, 'accepted');
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        $row = DB::table('native_scope_assignments')->sole();
        self::assertSame(['1', '3', self::NODE, 1_789_999_990], [
            NativeRuntimeStatusRow::string($row, 'assignment_epoch'),
            NativeRuntimeStatusRow::string($row, 'ownership_generation'),
            NativeRuntimeStatusRow::string($row, 'node_identity'),
            NativeRuntimeStatusRow::int($row, 'assigned_at'),
        ]);

        $this->assertResult($this->send(self::RUNTIME, $runtime, self::NODE), 'accepted');
        self::assertNotNull($this->readModel()->routable(self::WORLD, self::CHANNEL, 1_790_000_016));
    }

    public function test_the_assignment_key_only_moves_forward_and_an_equal_key_must_replay_identically(): void
    {
        $this->assertResult($this->send(self::ASSIGN, $this->wire()), 'accepted');
        $this->assertResult($this->send(self::ASSIGN, $this->wire()), 'accepted');
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(['node_identity' => self::NODE_B])), 409);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(['assigned_at' => '1789999991'])), 409);
        $this->assertResult($this->send(self::ASSIGN, $this->wire(['ownership_generation' => '2', 'node_identity' => self::NODE_B])), 'superseded');
        self::assertSame(self::NODE, NativeRuntimeStatusRow::string(DB::table('native_scope_assignments')->sole(), 'node_identity'));

        // Node replaced with a higher generation: the old node's report stops routing.
        $this->assertResult($this->send(self::RUNTIME, $this->runtimeWire(), self::NODE), 'accepted');
        $this->assertResult($this->send(self::ASSIGN, $this->wire(['ownership_generation' => '4', 'node_identity' => self::NODE_B])), 'accepted');
        self::assertSame('4', NativeRuntimeStatusRow::string(DB::table('native_scope_assignments')->sole(), 'ownership_generation'));
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $this->readModel()->evidence(self::WORLD, self::CHANNEL, 1_790_000_016));
    }

    public function test_an_epoch_raise_invalidates_every_lower_epoch_scope_and_refuses_lower_epochs(): void
    {
        $this->assertResult($this->send(self::ASSIGN, $this->wire()), 'accepted');
        $this->assertResult($this->send(self::RUNTIME, $this->runtimeWire(), self::NODE), 'accepted');

        $this->assertResult($this->send(self::ASSIGN, $this->wire(['assignment_epoch' => '2', 'channel_id' => self::OTHER_CHANNEL, 'ownership_generation' => '1', 'node_identity' => self::NODE_B])), 'accepted');
        self::assertSame(NativeRuntimeStatusReadModel::INVALID, $this->readModel()->evidence(self::WORLD, self::CHANNEL, 1_790_000_016));
        $this->assertRefused($this->send(self::RUNTIME, $this->runtimeWire(['observed_at' => '1790000016']), self::NODE), 409);

        // A delayed lower-epoch report is never stored, even with a higher generation.
        $this->assertResult($this->send(self::ASSIGN, $this->wire(['ownership_generation' => '9'])), 'superseded');
        self::assertSame('3', NativeRuntimeStatusRow::string(DB::table('native_scope_assignments')->where('channel_id', self::CHANNEL)->sole(), 'ownership_generation'));

        $this->assertResult($this->send(self::ASSIGN, $this->wire(['assignment_epoch' => '2', 'ownership_generation' => '1'])), 'accepted');
        $this->assertResult($this->send(self::RUNTIME, $this->runtimeWire([
            'assignment_epoch' => '2',
            'scope_ownership_generation' => '1',
            'decision_identity' => 'runtime-readiness:0a:1:7:true',
        ]), self::NODE), 'accepted');
        self::assertSame(NativeRuntimeStatusReadModel::FRESH, $this->readModel()->evidence(self::WORLD, self::CHANNEL, 1_790_000_016));
    }

    public function test_only_the_ownership_authority_may_report_assignments_of_its_scopes_to_configured_nodes(): void
    {
        foreach ([self::NODE, self::EVIDENCE, 'CN=unknown'] as $identity) {
            $this->assertRefused($this->send(self::ASSIGN, $this->wire(), $identity), 401);
        }
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(), self::OPS, ['SSL_CLIENT_VERIFY' => 'NONE']), 401);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(), self::OPS, ['SSL_PROTOCOL' => 'TLSv1.2']), 401);
        $this->assertRefused($this->send(self::RUNTIME, $this->runtimeWire(), self::OPS), 401);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(['channel_id' => '01934f10-7c03-7001-805b-3b1122334403'])), 401);

        // node_identity must be a runtime-status identity configured for the scope.
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(['node_identity' => 'CN=node-z.runtime-status'])), 409);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(['channel_id' => self::OTHER_CHANNEL])), 409);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(['node_identity' => self::OPS])), 409);
        self::assertFalse(DB::table('native_scope_assignments')->exists());
    }

    public function test_an_identity_reused_across_purposes_fails_closed_on_both_routes(): void
    {
        $scopes = [self::WORLD.'/'.self::CHANNEL];
        config(['game-auth.native_scope_assignment.identities' => [self::OPS => $scopes, self::NODE => $scopes]]);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire()), 503);
        $this->assertRefused($this->send(self::RUNTIME, $this->runtimeWire(), self::NODE), 503);

        config(['game-auth.native_scope_assignment.identities' => [self::EVIDENCE => $scopes]]);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire(), self::EVIDENCE), 503);

        // The node identity is checked against the runtime-status configuration, so it must be valid too.
        config(['game-auth.native_scope_assignment.identities' => [self::OPS => $scopes], 'game-auth.native_runtime_status.enabled' => false]);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire()), 503);

        // Refused even while the other purpose is switched off.
        config(['game-auth.native_runtime_status.enabled' => true]);
        config(['game-auth.native_scope_assignment.enabled' => false, 'game-auth.native_scope_assignment.identities' => [self::NODE => $scopes]]);
        $this->assertRefused($this->send(self::RUNTIME, $this->runtimeWire(), self::NODE), 503);
    }

    public function test_reports_outside_the_game_grammar_are_malformed(): void
    {
        $wire = $this->wire();
        $bodies = [
            'unknown member' => $this->wire(['host' => 'play.oteryn.test']),
            'missing member' => $this->wire([], ['assigned_at']),
            'null member' => $this->wire(['node_identity' => null]),
            'nested member' => $this->wire(['node_identity' => [self::NODE]]),
            'duplicate member' => str_replace('"assigned_at"', '"assigned_at":"1","assigned_at"', $wire),
            'escaped duplicate member' => str_replace('"assigned_at"', '"assigned_at":"1","assigned_at"', $wire),
            'zero epoch' => $this->wire(['assignment_epoch' => '0']),
            'integer generation' => $this->wire(['ownership_generation' => 3]),
            'leading zero generation' => $this->wire(['ownership_generation' => '03']),
            'uppercase world_id' => $this->wire(['world_id' => strtoupper(self::WORLD)]),
            'control character in node_identity' => $this->wire(['node_identity' => "CN=node-a\n"]),
            'long node_identity' => $this->wire(['node_identity' => str_repeat('n', 129)]),
            'negative assigned_at' => $this->wire(['assigned_at' => '-1']),
            'runtime operation' => $this->wire(['operation' => 'ReportRuntimeStatusV1']),
            'contract_version 2' => $this->wire(['contract_version' => 2]),
            'trailing data' => $wire.'{}',
        ];
        foreach ($bodies as $case => $body) {
            $this->assertRefused($this->send(self::ASSIGN, $body), 400, $case);
        }
        self::assertFalse(DB::table('native_scope_assignments')->exists());
    }

    public function test_early_http_bounds_apply_to_both_routes(): void
    {
        foreach ([[self::ASSIGN, $this->wire(), self::OPS], [self::RUNTIME, $this->runtimeWire(), self::NODE]] as [$path, $body, $identity]) {
            $this->assertRefused($this->send($path, $body, $identity, ['CONTENT_LENGTH' => '4096']), 413, $path);
            $this->assertRefused($this->send($path, $body.str_repeat(' ', 2048), $identity), 413, $path);
            $this->assertRefused($this->send($path, $body, $identity, ['HTTP_TRANSFER_ENCODING' => 'chunked']), 400, $path);
            $this->assertRefused($this->send($path, $body, $identity, ['HTTP_CONTENT_ENCODING' => 'gzip']), 400, $path);
            $this->assertRefused($this->send($path, $body, $identity, ['CONTENT_TYPE' => 'text/plain']), 400, $path);
            $this->assertRefused($this->send($path, $body, $identity, ['HTTP_X_PAD' => str_repeat('p', 2100)]), 431, $path);
        }
        self::assertFalse(DB::table('native_scope_assignments')->exists());
    }

    public function test_switch_is_default_off_rate_limited_and_rollback_is_refused_while_assignments_exist(): void
    {
        $default = require base_path('config/game-auth.php');
        self::assertIsArray($default);
        self::assertIsArray($default['native_scope_assignment']);
        self::assertFalse($default['native_scope_assignment']['enabled']);
        config(['game-auth.native_scope_assignment.enabled' => null]);
        $this->assertRefused($this->send(self::ASSIGN, $this->wire()), 503);
        config(['game-auth.native_scope_assignment.enabled' => true, 'game-auth.native_scope_assignment.requests_per_minute' => 1]);

        $this->assertResult($this->send(self::ASSIGN, $this->wire()), 'accepted');
        $limited = $this->send(self::ASSIGN, $this->wire());
        $this->assertRefused($limited, 429);
        self::assertNotNull($limited->headers->get('Retry-After'));

        $migration = require database_path('migrations/2026_09_30_180000_add_native_assignment_epoch_lock.php');
        self::assertIsObject($migration);
        try {
            (new ReflectionMethod($migration, 'down'))->invoke($migration);
            self::fail('Rollback must be refused while scope assignments exist.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('rollback refused', $exception->getMessage());
        }
        self::assertSame(1, DB::table('native_assignment_epoch_locks')->count());
    }

    /**
     * @param  array<string, mixed>  $changes
     * @param  list<string>  $remove
     */
    private function wire(array $changes = [], array $remove = []): string
    {
        $body = array_merge([
            'contract_version' => 1,
            'operation' => 'ReportScopeAssignmentV1',
            'assignment_epoch' => '1',
            'world_id' => self::WORLD,
            'channel_id' => self::CHANNEL,
            'ownership_generation' => '3',
            'node_identity' => self::NODE,
            'assigned_at' => '1789999990',
        ], $changes);

        return json_encode(array_diff_key($body, array_flip($remove)), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    /** @param array<string, mixed> $changes */
    private function runtimeWire(array $changes = []): string
    {
        $raw = file_get_contents(base_path('tests/Fixtures/GameAuth/native-runtime-status-v1/report.json'));
        self::assertIsString($raw);
        $decoded = json_decode($raw, true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($decoded);

        return json_encode(array_merge($decoded, $changes), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    /**
     * @param  array<string, string>  $server
     * @return TestResponse<Response>
     */
    private function send(string $path, string $body, string $identity = self::OPS, array $server = []): TestResponse
    {
        return $this->call('POST', $path, [], [], [], $server + [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => $identity,
        ], $body);
    }

    /** @param TestResponse<Response> $response */
    private function assertResult(TestResponse $response, string $result): void
    {
        self::assertSame('{"contract_version":1,"result":"'.$result.'"}', $response->getContent());
        $response->assertOk();
    }

    /** @param TestResponse<Response> $response */
    private function assertRefused(TestResponse $response, int $status, string $case = ''): void
    {
        self::assertSame($status, $response->getStatusCode(), $case);
        self::assertSame('', $response->getContent(), $case);
    }

    private function readModel(): NativeRuntimeStatusReadModel
    {
        return app(NativeRuntimeStatusReadModel::class);
    }
}
