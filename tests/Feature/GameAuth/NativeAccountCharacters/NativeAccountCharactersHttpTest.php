<?php

namespace Tests\Feature\GameAuth\NativeAccountCharacters;

use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class NativeAccountCharactersHttpTest extends TestCase
{
    use RefreshDatabase;

    private const IDENTITY = 'CN=character-authority-projection';

    private const AUTHORITY = 'oteryn:character-authority:primary';

    protected function setUp(): void
    {
        parent::setUp();
        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.publishers' => [self::IDENTITY => self::AUTHORITY],
            'game-auth.native_account_characters.freshness_seconds' => 30,
            'game-auth.native_account_characters.clock_uncertainty_seconds' => 1,
            'game-auth.native_account_characters.requests_per_minute' => 120,
        ]);
        $this->travelTo(CarbonImmutable::createFromTimestamp(1_790_000_020));
    }

    public function test_snapshot_and_watermark_routes_use_dedicated_mtls_and_exact_ack(): void
    {
        $snapshot = $this->publish('/internal/v1/game-auth/native-account-characters', $this->snapshotWire());
        $snapshot->assertOk();
        self::assertSame('{"contract_version":1,"result":"accepted"}', $snapshot->getContent());
        self::assertStringContainsString('no-store', (string) $snapshot->headers->get('Cache-Control'));

        $watermark = $this->publish('/internal/v1/game-auth/native-account-characters/watermark', $this->watermarkWire());
        $watermark->assertOk();
        self::assertSame('{"contract_version":1,"result":"accepted"}', $watermark->getContent());
    }

    public function test_transport_refuses_wrong_tls_identity_content_type_and_oversized_body(): void
    {
        $this->assertEmptyFailure($this->publish(
            '/internal/v1/game-auth/native-account-characters',
            $this->snapshotWire(),
            'CN=other',
        ), 401);
        $this->assertEmptyFailure($this->publish(
            '/internal/v1/game-auth/native-account-characters',
            $this->snapshotWire(),
            self::IDENTITY,
            ['SSL_PROTOCOL' => 'TLSv1.2'],
        ), 401);
        $this->assertEmptyFailure($this->call(
            'POST',
            '/internal/v1/game-auth/native-account-characters',
            [],
            [],
            [],
            $this->server(['CONTENT_TYPE' => 'text/plain']),
            $this->snapshotWire(),
        ), 400);
        $this->assertEmptyFailure($this->publish(
            '/internal/v1/game-auth/native-account-characters/watermark',
            str_repeat('x', 513),
        ), 413);
    }

    public function test_switch_is_default_off_and_rate_limit_is_per_projection_identity(): void
    {
        config(['game-auth.native_account_characters.enabled' => false]);
        $this->assertEmptyFailure($this->publish('/internal/v1/game-auth/native-account-characters', $this->snapshotWire()), 503);

        config([
            'game-auth.native_account_characters.enabled' => true,
            'game-auth.native_account_characters.requests_per_minute' => 1,
        ]);
        $this->publish('/internal/v1/game-auth/native-account-characters', $this->snapshotWire())->assertOk();
        $limited = $this->publish('/internal/v1/game-auth/native-account-characters', $this->snapshotWire());
        $this->assertEmptyFailure($limited, 429);
        self::assertNotNull($limited->headers->get('Retry-After'));
    }

    /** @param array<string, string> $server */
    private function publish(string $path, string $body, string $identity = self::IDENTITY, array $server = []): TestResponse
    {
        return $this->call('POST', $path, [], [], [], $this->server($server + [
            'SSL_CLIENT_S_DN' => $identity,
        ]), $body);
    }

    /** @param array<string, string> $changes @return array<string, string> */
    private function server(array $changes = []): array
    {
        return $changes + [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::IDENTITY,
        ];
    }

    private function snapshotWire(): string
    {
        return json_encode([
            'contract_version' => 1,
            'operation' => 'PublishAccountCharactersV1',
            'source_authority' => self::AUTHORITY,
            'account_id' => '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6',
            'projection_epoch' => '1',
            'projection_revision' => '42',
            'source_observed_at' => '1790000000',
            'characters' => [[
                'character_id' => '01934f10-7c04-7001-805b-3b1122334401',
                'world_id' => '01934f10-7c02-7001-805b-3b1122334401',
                'name' => 'Aldric',
                'availability' => 'AVAILABLE',
            ]],
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    private function watermarkWire(): string
    {
        return json_encode([
            'contract_version' => 1,
            'operation' => 'PublishProjectionWatermarkV1',
            'source_authority' => self::AUTHORITY,
            'projection_epoch' => '1',
            'complete_through' => '1790000010',
            'observed_at' => '1790000018',
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }

    /** @param TestResponse<Response> $response */
    private function assertEmptyFailure(TestResponse $response, int $status): void
    {
        self::assertSame($status, $response->getStatusCode());
        self::assertSame('', $response->getContent());
    }
}
