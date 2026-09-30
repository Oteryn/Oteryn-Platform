<?php

namespace Tests\Feature\ProductsEntitlements;

use App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings;
use App\Identity\Models\Identity;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use App\ProductsEntitlements\Premium\PremiumTimeLedger;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

/**
 * Producer contract test for OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md. Every issued snapshot is checked against
 * the shared fixtures in docs/contracts/fixtures/premium-snapshot-v1 that Oteryn-v2 reuses as its consumer fixture.
 */
final class PremiumSnapshotEndpointTest extends TestCase
{
    use RefreshDatabase;

    private const URI = '/internal/v1/products-entitlements/premium-snapshots/read';

    private const PEER = 'CN=oteryn-game-premium-snapshot';

    private const REVISION = '0123456789abcdef0123456789abcdef01234567';

    private const FIXTURES = 'docs/contracts/fixtures/premium-snapshot-v1/';

    private const DAY = 86400;

    private const REASON = 'Operator grant for closed test cohort';

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-10-01T12:00:00Z');
        config([
            'hashing.driver' => 'bcrypt',
            'products-entitlements.premium_snapshot.enabled' => true,
            'products-entitlements.premium_snapshot.mtls_client_identity' => self::PEER,
            'products-entitlements.premium_snapshot.requests_per_minute' => '1200',
            'products-entitlements.premium_snapshot.producer_revision' => self::REVISION,
        ]);
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_shared_fixtures_are_accepted_and_every_manifest_invalid_case_is_rejected(): void
    {
        $manifest = $this->fixture('manifest.json');
        self::assertSame('OTV2-PREMIUM-DELIVERY', $manifest['coordination_id']);
        $policy = $this->arrayOf($manifest['policy']);
        self::assertSame(PremiumTimeContract::MAX_AUTHORITY_LEASE_SECONDS, $policy['max_authority_lease_seconds']);
        self::assertSame(PremiumTimeContract::MAX_CLOCK_SKEW_SECONDS, $policy['max_clock_skew_seconds']);
        self::assertSame('DENY', $policy['stale_within_bound']);
        self::assertSame(PremiumTimeContract::MAX_RESPONSE_BYTES, $policy['max_response_bytes']);
        self::assertSame(PremiumTimeContract::MAX_REQUEST_BYTES, $policy['max_request_bytes']);
        $request = $this->fixture('request.valid.json');
        $requestSchema = $this->fixture('request.schema.json');

        self::assertSame([], $this->schemaErrors($request, $requestSchema));
        $valid = $this->arrayOf($manifest['valid_snapshots']);
        self::assertCount(5, $valid);
        foreach ($valid as $entry) {
            $file = $this->stringOf($this->arrayOf($entry)['file']);
            self::assertSame([], $this->contractErrors($this->fixture($file), $request), $file);
        }
        foreach ($this->arrayOf($manifest['invalid_snapshots']) as $entry) {
            $entry = $this->arrayOf($entry);
            self::assertNotSame([], $this->contractErrors($this->arrayOf($entry['body']), $request), $this->stringOf($entry['case']));
        }
        foreach ($this->arrayOf($manifest['invalid_requests']) as $entry) {
            $entry = $this->arrayOf($entry);
            self::assertNotSame([], $this->schemaErrors($entry['body'], $requestSchema), $this->stringOf($entry['case']));
        }
    }

    public function test_none_snapshot_is_exact_non_cacheable_and_allocates_one_revision_per_read(): void
    {
        $player = $this->identity('player@example.test');
        $nonce = $this->nonce();

        $response = $this->read($player->account_id, $nonce)->assertOk();
        $this->assertPrivateNoStore($response);
        $snapshot = $this->validSnapshot($response, $player->account_id, $nonce);

        self::assertSame([
            'schema', 'producer_revision', 'nonce', 'account_id', 'product_id', 'product_version', 'entitlement_id',
            'entitlement_state', 'lifecycle_revision', 'authority_revision', 'effective_from', 'effective_until',
            'authority_issued_at', 'authority_valid_until', 'refresh_after',
        ], array_keys($snapshot));
        self::assertSame(PremiumTimeContract::STATE_NONE, $snapshot['entitlement_state']);
        self::assertNull($snapshot['entitlement_id']);
        self::assertSame(0, $snapshot['lifecycle_revision']);
        self::assertSame(1, $snapshot['authority_revision']);
        self::assertSame(self::REVISION, $snapshot['producer_revision']);
        self::assertSame('2026-10-01T12:00:00Z', $snapshot['authority_issued_at']);
        self::assertSame('2026-10-01T13:00:00Z', $snapshot['authority_valid_until']);
        self::assertSame('2026-10-01T12:40:00Z', $snapshot['refresh_after']);

        $second = $this->validSnapshot($this->read($player->account_id, $this->nonce())->assertOk(), $player->account_id);
        self::assertSame(2, $second['authority_revision']);
        self::assertSame(0, DB::table('premium_time_entitlements')->count());
    }

    public function test_active_expired_and_revoked_snapshots_follow_lease_refresh_and_revision_rules(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $start = now()->getTimestamp();
        $entitlement = $this->ledger()->grant($operator, $player, 30, self::REASON, $this->requestId());

        $active = $this->validSnapshot($this->read($player->account_id)->assertOk(), $player->account_id);
        self::assertSame(PremiumTimeContract::STATE_ACTIVE, $active['entitlement_state']);
        self::assertSame($entitlement->id, $active['entitlement_id']);
        self::assertSame(1, $active['lifecycle_revision']);
        self::assertSame('2026-10-01T12:00:00Z', $active['effective_from']);
        self::assertSame('2026-10-31T12:00:00Z', $active['effective_until']);
        self::assertSame('2026-10-01T13:00:00Z', $active['authority_valid_until']);
        self::assertSame('2026-10-01T12:40:00Z', $active['refresh_after']);

        // 30 minutes before the end the lease is clipped to effective_until and refresh moves to two thirds of it.
        Carbon::setTestNow(Carbon::createFromTimestampUTC($start + 30 * self::DAY - 1800));
        $clipped = $this->validSnapshot($this->read($player->account_id)->assertOk(), $player->account_id);
        self::assertSame('2026-10-31T12:00:00Z', $clipped['authority_valid_until']);
        self::assertSame('2026-10-31T11:50:00Z', $clipped['refresh_after']);
        self::assertGreaterThan($active['authority_revision'], $clipped['authority_revision']);

        // Issued exactly at effective_until: EXPIRED, same lifecycle revision, full-length negative lease.
        Carbon::setTestNow(Carbon::createFromTimestampUTC($start + 30 * self::DAY));
        $expired = $this->validSnapshot($this->read($player->account_id)->assertOk(), $player->account_id);
        self::assertSame(PremiumTimeContract::STATE_EXPIRED, $expired['entitlement_state']);
        self::assertSame(1, $expired['lifecycle_revision']);
        self::assertSame('2026-10-31T13:00:00Z', $expired['authority_valid_until']);

        $this->ledger()->grant($operator, $player, 10, self::REASON, $this->requestId());
        Carbon::setTestNow(Carbon::createFromTimestampUTC($start + 31 * self::DAY));
        $this->ledger()->revoke($operator, $player, 'Granted to the wrong account', $this->requestId());
        $revoked = $this->validSnapshot($this->read($player->account_id)->assertOk(), $player->account_id);
        self::assertSame(PremiumTimeContract::STATE_REVOKED, $revoked['entitlement_state']);
        self::assertSame(3, $revoked['lifecycle_revision']);
        self::assertSame('2026-11-01T12:00:00Z', $revoked['effective_until']);
        self::assertSame($entitlement->id, $revoked['entitlement_id']);

        $revisions = array_column([$active, $clipped, $expired, $revoked], 'authority_revision');
        for ($i = 1; $i < count($revisions); $i++) {
            self::assertGreaterThan($revisions[$i - 1], $revisions[$i]);
        }
    }

    public function test_authority_revisions_are_per_account_and_unknown_accounts_allocate_nothing(): void
    {
        [$first, $second] = [$this->identity('first@example.test'), $this->identity('second@example.test')];

        self::assertSame(1, $this->validSnapshot($this->read($first->account_id)->assertOk(), $first->account_id)['authority_revision']);
        self::assertSame(2, $this->validSnapshot($this->read($first->account_id)->assertOk(), $first->account_id)['authority_revision']);
        self::assertSame(1, $this->validSnapshot($this->read($second->account_id)->assertOk(), $second->account_id)['authority_revision']);

        $before = DB::table('premium_time_authority')->count();
        $this->read(strtolower((string) Str::uuid()))->assertNotFound()->assertContent('');
        self::assertSame($before, DB::table('premium_time_authority')->count());
    }

    public function test_peer_provenance_is_exact_and_misconfiguration_fails_closed(): void
    {
        $player = $this->identity('player@example.test');
        $body = $this->body($player->account_id, $this->nonce());

        $this->assertPrivateNoStore($this->call('POST', self::URI, [], [], [], ['CONTENT_TYPE' => 'application/json'], $body)
            ->assertUnauthorized()->assertContent(''));
        foreach ([
            ['SSL_CLIENT_S_DN' => 'CN=oteryn-game-character-authority'],
            ['SSL_CLIENT_VERIFY' => 'FAILED:unable to verify'],
            ['SSL_PROTOCOL' => 'TLSv1.2'],
            ['SSL_CLIENT_S_DN' => self::PEER.' '],
        ] as $override) {
            $this->rawRead($body, array_merge($this->peer(), $override))->assertUnauthorized()->assertContent('');
        }

        foreach ([
            ['products-entitlements.premium_snapshot.enabled' => false],
            ['products-entitlements.premium_snapshot.enabled' => 'true'],
            ['products-entitlements.premium_snapshot.mtls_client_identity' => null],
            ['products-entitlements.premium_snapshot.mtls_client_identity' => ''],
            ['products-entitlements.premium_snapshot.producer_revision' => null],
            ['products-entitlements.premium_snapshot.producer_revision' => strtoupper(self::REVISION)],
            ['products-entitlements.premium_snapshot.requests_per_minute' => '0'],
            ['game-auth.character_bootstrap_intent.mtls_client_identity' => self::PEER],
            ['game-auth.native_evidence.mtls_client_identity' => self::PEER],
            ['game-auth.native_runtime_status.identities' => json_encode([self::PEER => ['a/b']])],
        ] as $configuration) {
            $restore = [];
            foreach (array_keys($configuration) as $key) {
                $restore[$key] = config($key);
            }
            config($configuration);
            $this->rawRead($body, $this->peer())->assertStatus(503)->assertContent('');
            config($restore);
        }

        self::assertSame(0, DB::table('premium_time_authority')->count());
        $this->rawRead($body, $this->peer())->assertOk();
    }

    public function test_premium_snapshot_identity_is_refused_by_runtime_status_purpose_separation(): void
    {
        $scope = ['01890f4e-7c00-7000-8000-000000000002/01890f4e-7c00-7000-8000-000000000003'];

        // Positive control: the same scope under a runtime-only identity is accepted.
        self::assertNotNull(NativeRuntimeStatusSettings::identities(json_encode(['CN=oteryn-game-node-1' => $scope]), 'native_scope_assignment'));
        self::assertNull(NativeRuntimeStatusSettings::identities(json_encode([self::PEER => $scope]), 'native_scope_assignment'));
    }

    public function test_malformed_requests_are_rejected_without_allocating_authority(): void
    {
        $player = $this->identity('player@example.test');
        $account = $player->account_id;
        $nonce = $this->nonce();
        $schema = PremiumTimeContract::REQUEST_SCHEMA;

        $raws = [
            '',
            '[]',
            '{}',
            'not json',
            '{"schema":"'.$schema.'","account_id":"'.$account.'","nonce":"'.$nonce.'","nonce":"'.$nonce.'"}',
            '{"schema":"'.$schema.'","account_id":{"id":"'.$account.'"},"nonce":"'.$nonce.'"}',
            '{"schema":"'.$schema.'","account_id":"'.$account.'","nonce":"'.$nonce.'"}'.str_repeat(' ', 256),
        ];
        foreach ($this->arrayOf($this->fixture('manifest.json')['invalid_requests']) as $invalid) {
            $raws[] = json_encode($this->arrayOf($invalid)['body'], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        }

        foreach ($raws as $raw) {
            $this->rawRead($raw, $this->peer())->assertStatus(400)->assertContent('');
        }
        self::assertSame(0, DB::table('premium_time_authority')->count());

        // The shared valid request fixture is accepted byte for byte (its account is unknown here).
        $this->rawRead((string) file_get_contents(base_path(self::FIXTURES.'request.valid.json')), $this->peer())
            ->assertNotFound();
    }

    public function test_requests_are_bounded_per_peer(): void
    {
        config(['products-entitlements.premium_snapshot.requests_per_minute' => '2']);
        $player = $this->identity('player@example.test');

        $this->read($player->account_id)->assertOk();
        $this->read($player->account_id)->assertOk();
        $this->read($player->account_id)->assertStatus(429)->assertContent('');
        self::assertSame(2, (int) DB::table('premium_time_authority')->value('authority_revision'));
    }

    public function test_invalid_durable_entitlement_state_is_unavailable(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $this->ledger()->grant($operator, $player, 1, self::REASON, $this->requestId());

        DB::table('premium_time_entitlements')->update(['product_id' => 'oteryn.other']);
        $this->read($player->account_id)->assertStatus(503)->assertContent('');

        DB::table('premium_time_entitlements')->update(['product_id' => PremiumTimeContract::PRODUCT_ID, 'state' => 'SUSPENDED']);
        $this->read($player->account_id)->assertStatus(503)->assertContent('');

        DB::table('premium_time_entitlements')->update(['state' => PremiumTimeContract::STORED_ACTIVE]);
        DB::table('premium_time_authority')->update(['authority_revision' => PremiumTimeContract::MAX_REVISION]);
        $this->read($player->account_id)->assertStatus(503)->assertContent('');
    }

    /**
     * Assert the response is a contract-valid snapshot bound to the request and return it.
     *
     * @param  TestResponse<Response>  $response
     * @return array<mixed>
     */
    private function validSnapshot(TestResponse $response, string $accountId, ?string $nonce = null): array
    {
        $raw = (string) $response->getContent();
        self::assertLessThanOrEqual(PremiumTimeContract::MAX_RESPONSE_BYTES, strlen($raw));
        $snapshot = json_decode($raw, true, 3, JSON_THROW_ON_ERROR);
        self::assertIsArray($snapshot);
        $request = ['account_id' => $accountId, 'nonce' => $nonce ?? $snapshot['nonce']];
        self::assertSame([], $this->contractErrors($snapshot, $request));

        return $snapshot;
    }

    /**
     * Snapshot schema plus the manifest cross_field_rules (contract 5.1, 5.2, 5.3).
     *
     * @param  array<mixed>  $snapshot
     * @param  array<mixed>  $request
     * @return list<string>
     */
    private function contractErrors(array $snapshot, array $request): array
    {
        $errors = $this->schemaErrors($snapshot, $this->fixture('snapshot.schema.json'));
        if ($errors !== []) {
            return $errors;
        }
        if (strlen(json_encode($snapshot, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES)) > PremiumTimeContract::MAX_RESPONSE_BYTES) {
            $errors[] = 'size';
        }
        if ($snapshot['nonce'] !== $request['nonce'] || $snapshot['account_id'] !== $request['account_id']) {
            $errors[] = 'request binding';
        }
        $issued = $this->time($snapshot['authority_issued_at']);
        $validUntil = $this->time($snapshot['authority_valid_until']);
        $refresh = $this->time($snapshot['refresh_after']);
        $state = $snapshot['entitlement_state'];
        $from = $snapshot['effective_from'] === null ? null : $this->time($snapshot['effective_from']);
        $until = $snapshot['effective_until'] === null ? null : $this->time($snapshot['effective_until']);
        if ($from !== null && $until !== null && $from >= $until) {
            $errors[] = 'interval order';
        }
        if (! ($issued <= $refresh && $refresh < $validUntil && $validUntil <= $issued + PremiumTimeContract::MAX_AUTHORITY_LEASE_SECONDS)) {
            $errors[] = 'lease ordering';
        }
        $expectedValidUntil = in_array($state, ['ACTIVE', 'NOT_YET_EFFECTIVE'], true)
            ? min($issued + PremiumTimeContract::MAX_AUTHORITY_LEASE_SECONDS, (int) $until)
            : $issued + PremiumTimeContract::MAX_AUTHORITY_LEASE_SECONDS;
        if ($validUntil !== $expectedValidUntil) {
            $errors[] = 'authority_valid_until';
        }
        if ($refresh !== $issued + intdiv(2 * ($validUntil - $issued), 3)) {
            $errors[] = 'refresh_after';
        }
        if (($state === 'ACTIVE' && ! ($from <= $issued && $issued < $until))
            || ($state === 'NOT_YET_EFFECTIVE' && ! ($issued < $from))
            || ($state === 'EXPIRED' && ! ($issued >= $until))) {
            $errors[] = 'state window';
        }

        return $errors;
    }

    /**
     * The JSON Schema subset used by the shared fixtures: type, const, enum, pattern, minimum, maximum, required,
     * properties, additionalProperties false and if/then/else.
     *
     * @param  array<mixed>  $schema
     * @return list<string>
     */
    private function schemaErrors(mixed $value, array $schema, string $path = '$'): array
    {
        $errors = [];
        if (array_key_exists('const', $schema) && $value !== $schema['const']) {
            $errors[] = "{$path} const";
        }
        if (is_array($schema['enum'] ?? null) && ! in_array($value, $schema['enum'], true)) {
            $errors[] = "{$path} enum";
        }
        if (isset($schema['type'])) {
            $types = is_array($schema['type']) ? $schema['type'] : [$schema['type']];
            $matches = array_filter($types, fn (mixed $type): bool => match ($type) {
                'object' => is_array($value) && ($value === [] || ! array_is_list($value)),
                'string' => is_string($value),
                'integer' => is_int($value),
                'null' => $value === null,
                default => false,
            });
            if ($matches === []) {
                return ["{$path} type"];
            }
        }
        $pattern = $schema['pattern'] ?? null;
        if (is_string($pattern) && is_string($value) && preg_match('~'.$pattern.'~D', $value) !== 1) {
            $errors[] = "{$path} pattern";
        }
        $minimum = $schema['minimum'] ?? null;
        $maximum = $schema['maximum'] ?? null;
        if (is_int($value) && is_int($minimum) && $value < $minimum) {
            $errors[] = "{$path} minimum";
        }
        if (is_int($value) && is_int($maximum) && $value > $maximum) {
            $errors[] = "{$path} maximum";
        }
        if (! is_array($value) || ($value !== [] && array_is_list($value))) {
            return $errors;
        }

        $required = is_array($schema['required'] ?? null) ? $schema['required'] : [];
        foreach ($required as $member) {
            if (! is_string($member) || ! array_key_exists($member, $value)) {
                $errors[] = "{$path} required";
            }
        }
        $properties = is_array($schema['properties'] ?? null) ? $schema['properties'] : [];
        foreach ($value as $member => $memberValue) {
            $memberSchema = $properties[$member] ?? null;
            if (is_array($memberSchema)) {
                array_push($errors, ...$this->schemaErrors($memberValue, $memberSchema, "{$path}.{$member}"));
            } elseif (($schema['additionalProperties'] ?? true) === false) {
                $errors[] = "{$path}.{$member} additional";
            }
        }
        if (is_array($schema['if'] ?? null)) {
            $branch = $this->schemaErrors($value, $schema['if'], $path) === [] ? ($schema['then'] ?? []) : ($schema['else'] ?? []);
            array_push($errors, ...$this->schemaErrors($value, is_array($branch) ? $branch : [], $path));
        }

        return $errors;
    }

    private function time(mixed $value): int
    {
        $parsed = Carbon::createFromFormat('Y-m-d\TH:i:s\Z', $this->stringOf($value), 'UTC');
        self::assertInstanceOf(Carbon::class, $parsed);

        return $parsed->getTimestamp();
    }

    /** @return array<mixed> */
    private function fixture(string $name): array
    {
        return $this->arrayOf(json_decode((string) file_get_contents(base_path(self::FIXTURES.$name)), true, 64, JSON_THROW_ON_ERROR));
    }

    /** @return array<mixed> */
    private function arrayOf(mixed $value): array
    {
        self::assertIsArray($value);

        return $value;
    }

    private function stringOf(mixed $value): string
    {
        self::assertIsString($value);

        return $value;
    }

    private function ledger(): PremiumTimeLedger
    {
        return app(PremiumTimeLedger::class);
    }

    /** @return TestResponse<Response> */
    private function read(string $accountId, ?string $nonce = null): TestResponse
    {
        return $this->rawRead($this->body($accountId, $nonce ?? $this->nonce()), $this->peer());
    }

    /**
     * @param  array<string, string>  $server
     * @return TestResponse<Response>
     */
    private function rawRead(string $raw, array $server): TestResponse
    {
        return $this->call('POST', self::URI, [], [], [], $server, $raw);
    }

    private function body(string $accountId, string $nonce): string
    {
        return json_encode([
            'schema' => PremiumTimeContract::REQUEST_SCHEMA,
            'account_id' => $accountId,
            'nonce' => $nonce,
        ], JSON_THROW_ON_ERROR);
    }

    /** @return array<string, string> */
    private function peer(): array
    {
        return [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
        ];
    }

    /** @param TestResponse<Response> $response */
    private function assertPrivateNoStore(TestResponse $response): void
    {
        $value = (string) $response->headers->get('Cache-Control');
        self::assertStringContainsString('no-store', $value);
        self::assertStringContainsString('private', $value);
    }

    private function nonce(): string
    {
        return bin2hex(random_bytes(16));
    }

    private function requestId(): string
    {
        return strtolower((string) Str::uuid());
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }
}
