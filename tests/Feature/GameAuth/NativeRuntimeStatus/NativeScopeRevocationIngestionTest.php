<?php

namespace Tests\Feature\GameAuth\NativeRuntimeStatus;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusReadModel;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Testing\TestResponse;
use LogicException;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/** Platform ingestion of Game `ReportScopeRevocationV1` contract §16.1. */
final class NativeScopeRevocationIngestionTest extends TestCase
{
    use RefreshDatabase;

    private const ASSIGN = '/internal/v1/game-auth/native-scope-assignments';
    private const REVOKE = '/internal/v1/game-auth/native-scope-revocations';
    private const RUNTIME = '/internal/v1/game-auth/native-runtime-status';
    private const OPS = 'CN=oteryn-game-ops.ownership-authority';
    private const NODE = 'CN=node-a.runtime-status';
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
            'game-auth.native_runtime_status.identities' => [self::NODE => [$scope]],
            'game-auth.native_runtime_status.freshness_seconds' => '15',
            'game-auth.native_runtime_status.clock_uncertainty_seconds' => '1',
            'game-auth.native_runtime_status.requests_per_minute' => '600',
            'game-auth.native_scope_assignment.enabled' => true,
            'game-auth.native_scope_assignment.identities' => json_encode([self::OPS => [$scope, $other]], JSON_THROW_ON_ERROR),
            'game-auth.native_scope_assignment.requests_per_minute' => '60',
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_016));
    }

    public function test_reports_outside_the_game_grammar_are_malformed(): void
    {
        $decoded = json_decode($this->revocationWire(1, 4), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($decoded);
        $missing = $decoded;
        unset($missing['revoked_at']);
        $bodies = [
            json_encode($missing, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES),
            $this->revocationWire(1, 4, override: ['extra' => 'member']),
            $this->revocationWire(1, 4, override: ['operation' => 'ReportScopeAssignmentV1']),
            $this->revocationWire(1, 4, override: ['assignment_epoch' => '0']),
            $this->revocationWire(1, 4, override: ['assignment_epoch' => '01']),
            $this->revocationWire(1, 4, override: ['world_id' => '01934f10-7c02-6001-805b-3b1122334401']),
            $this->revocationWire(1, 4, override: ['channel_id' => self::WORLD]),
            $this->revocationWire(1, 4, override: ['revoked_at' => 1_790_000_100]),
        ];
        foreach ($bodies as $body) {
            $this->assertRefused($this->send(self::REVOKE, $body), 400);
        }
        $this->assertRefused($this->send(self::REVOKE, $this->revocationWire(1, 4).str_repeat(' ', 2048)), 413);
    }

    public function test_a_revocation_supersedes_the_assignment_and_the_scope_routes_nowhere(): void
    {
        $this->assertResult($this->send(self::ASSIGN, $this->assignmentWire()), 'accepted');
        $this->assertResult($this->send(self::RUNTIME, $this->runtimeWire(), self::NODE), 'accepted');
        self::assertNotNull($this->readModel()->routable(self::WORLD, self::CHANNEL, 1_790_000_016));

        $this->assertResult($this->send(self::REVOKE, $this->revocationWire(1, 4)), 'accepted');
        self::assertNull($this->readModel()->routable(self::WORLD, self::CHANNEL, 1_790_000_016));
        $row = DB::table('native_scope_assignments')->sole();
        self::assertNull($row->node_identity);
        self::assertSame(1_790_000_100, (int) $row->revoked_at);
        self::assertTrue((bool) DB::table('native_runtime_status_reports')->value('invalid'));
        $this->assertRefused($this->send(self::RUNTIME, $this->runtimeWire(), self::NODE), 409);
    }

    public function test_revocation_ordering_replays_and_conflicts(): void
    {
        $this->assertResult($this->send(self::ASSIGN, $this->assignmentWire()), 'accepted');
        $this->assertResult($this->send(self::REVOKE, $this->revocationWire(1, 2)), 'superseded');
        $this->assertRefused($this->send(self::REVOKE, $this->revocationWire(1, 3)), 409);
        $this->assertResult($this->send(self::REVOKE, $this->revocationWire(1, 4)), 'accepted');
        $updatedAt = DB::table('native_scope_assignments')->value('updated_at');
        $this->assertResult($this->send(self::REVOKE, $this->revocationWire(1, 4)), 'accepted');
        self::assertEquals($updatedAt, DB::table('native_scope_assignments')->value('updated_at'));
        $this->assertRefused($this->send(self::REVOKE, $this->revocationWire(1, 4, 1_790_000_101)), 409);
        $this->assertRefused($this->send(self::ASSIGN, $this->assignmentWire(['ownership_generation' => '4'])), 409);

        $this->assertResult($this->send(self::ASSIGN, $this->assignmentWire(['ownership_generation' => '5'])), 'accepted');
        $runtime = $this->runtimeWire([
            'scope_ownership_generation' => '5',
            'decision_identity' => 'runtime-readiness:0a:5:7:true',
        ]);
        $this->assertResult($this->send(self::RUNTIME, $runtime, self::NODE), 'accepted');
        self::assertNotNull($this->readModel()->routable(self::WORLD, self::CHANNEL, 1_790_000_016));
        $this->assertResult($this->send(self::REVOKE, $this->revocationWire(1, 4)), 'superseded');
    }

    public function test_only_the_ownership_authority_may_revoke_its_scopes_and_the_switch_is_default_off(): void
    {
        $default = require base_path('config/game-auth.php');
        self::assertIsArray($default);
        self::assertIsArray($default['native_scope_assignment']);
        self::assertFalse($default['native_scope_assignment']['enabled']);

        $unknownScope = $this->revocationWire(1, 4, override: ['channel_id' => '01934f10-7c03-7001-805b-3b1122334403']);
        $this->assertRefused($this->send(self::REVOKE, $unknownScope), 401);
        $this->assertRefused($this->send(self::REVOKE, $this->revocationWire(1, 4), self::NODE), 401);

        config(['game-auth.native_scope_assignment.enabled' => null]);
        $response = $this->send(self::REVOKE, $this->revocationWire(1, 4));
        $this->assertRefused($response, 503);
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
    }

    public function test_rollback_is_refused_while_a_revocation_exists(): void
    {
        $this->assertResult($this->send(self::REVOKE, $this->revocationWire(1, 4)), 'accepted');
        $migration = require database_path('migrations/2026_11_05_090000_add_native_scope_revocations.php');
        self::assertIsObject($migration);

        $this->expectException(LogicException::class);
        $this->expectExceptionMessage('rollback refused: scope revocations exist');
        $migration->down();
    }

    /** @param array<string, mixed> $override */
    private function revocationWire(int $epoch, int $generation, int $revokedAt = 1_790_000_100, array $override = []): string
    {
        return json_encode(array_merge([
            'contract_version' => 1,
            'operation' => 'ReportScopeRevocationV1',
            'assignment_epoch' => (string) $epoch,
            'world_id' => self::WORLD,
            'channel_id' => self::CHANNEL,
            'ownership_generation' => (string) $generation,
            'revoked_at' => (string) $revokedAt,
        ], $override), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    /** @param array<string, mixed> $changes */
    private function assignmentWire(array $changes = []): string
    {
        return json_encode(array_merge([
            'contract_version' => 1,
            'operation' => 'ReportScopeAssignmentV1',
            'assignment_epoch' => '1',
            'world_id' => self::WORLD,
            'channel_id' => self::CHANNEL,
            'ownership_generation' => '3',
            'node_identity' => self::NODE,
            'assigned_at' => '1789999990',
        ], $changes), JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
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

    /** @param array<string, string> $server */
    private function send(string $path, string $body, string $identity = self::OPS, array $server = []): TestResponse
    {
        return $this->call('POST', $path, [], [], [], $server + [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => $identity,
        ], $body);
    }

    private function assertResult(TestResponse $response, string $result): void
    {
        self::assertSame('{"contract_version":1,"result":"'.$result.'"}', $response->getContent());
        $response->assertOk();
    }

    private function assertRefused(TestResponse $response, int $status): void
    {
        self::assertSame($status, $response->getStatusCode());
        self::assertSame('', $response->getContent());
    }

    private function readModel(): NativeRuntimeStatusReadModel
    {
        return app(NativeRuntimeStatusReadModel::class);
    }
}
