<?php

namespace Tests\Feature\CharacterAuthority;

use App\CharacterAuthority\CharacterCommandBacklogFull;
use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandConflict;
use App\CharacterAuthority\CharacterCommandState;
use App\CharacterAuthority\CharacterCommandUnavailable;
use App\CharacterAuthority\CharacterCommandWire;
use App\CharacterAuthority\CharacterTransferPurpose;
use App\CharacterAuthority\Http\CharacterCommandRoutes;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Route;
use Illuminate\Testing\TestResponse;
use InvalidArgumentException;
use LogicException;
use ReflectionMethod;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class CharacterCommandClientTest extends TestCase
{
    use RefreshDatabase;

    private const PEER = 'CN=oteryn-game-character-commands';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const OTHER_ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c7';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const NIL = '00000000-0000-0000-0000-000000000000';

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-10-11T12:00:00Z');
        Route::middleware('api')->group(fn () => CharacterCommandRoutes::register());
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_commands_are_disabled_by_default_and_routes_answer_unavailable(): void
    {
        self::assertFalse(config('character-authority.commands.enabled'));

        try {
            $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
            self::fail('A disabled command client must not store an intent.');
        } catch (CharacterCommandUnavailable) {
        }

        $response = $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest());
        $response->assertStatus(503);
        self::assertSame('', $response->getContent());
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        self::assertSame(0, DB::table('character_command_intents')->count());
    }

    public function test_create_intent_wire_is_exact_listed_and_read_byte_identical(): void
    {
        $this->enable();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');

        self::assertSame(CharacterCommandState::Pending, $status->state);
        self::assertSame(CharacterCommandWire::CREATE, $status->command);
        self::assertSame(Carbon::now()->getTimestamp() + 300, $status->expiresAtSource);

        $list = $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest());
        $list->assertOk();
        self::assertSame(
            '{"contract_version":1,"pending":[{"command":"CreateCharacter","operation_id":"'.$status->operationId.'","source_revision":"1"}]}',
            $list->getContent(),
        );
        self::assertStringContainsString('no-store', (string) $list->headers->get('Cache-Control'));

        $first = $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::CREATE, $status->operationId));
        $second = $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::CREATE, $status->operationId));
        $first->assertOk();
        self::assertSame($first->getContent(), $second->getContent());
        self::assertSame(DB::table('character_command_intents')->value('intent_json'), $first->getContent());

        $intent = json_decode((string) $first->getContent(), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($intent);
        self::assertSame([
            'contract_version', 'command', 'variant', 'issuer_authority', 'issuer_decision_id', 'source_revision',
            'operation_id', 'account_id', 'target_world_id', 'requested_name', 'issued_at_source', 'expires_at_source', 'audience',
        ], array_keys($intent));
        self::assertSame('PLATFORM_USER_CREATE', $intent['variant']);
        self::assertSame('OTERYN_PLATFORM_CHARACTER_AUTHORITY', $intent['issuer_authority']);
        self::assertSame('OTERYN_GAME_CHARACTER_AUTHORITY', $intent['audience']);
        self::assertSame('1', $intent['source_revision']);
        self::assertSame(self::ACCOUNT, $intent['account_id']);
        self::assertSame((string) Carbon::now()->getTimestamp(), $intent['issued_at_source']);
        self::assertSame((string) (Carbon::now()->getTimestamp() + 300), $intent['expires_at_source']);
        self::assertNotSame($status->operationId, $intent['issuer_decision_id']);
        self::assertStringNotContainsString(' ', (string) $first->getContent());
    }

    public function test_retry_with_the_same_idempotency_key_returns_the_same_operation_and_bytes(): void
    {
        $this->enable();
        $client = $this->client();
        $first = $client->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $bytes = DB::table('character_command_intents')->value('intent_json');

        Carbon::setTestNow('2026-10-11T12:00:20Z');
        $retry = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');

        self::assertSame($first->operationId, $retry->operationId);
        self::assertSame($first->expiresAtSource, $retry->expiresAtSource);
        self::assertSame(1, DB::table('character_command_intents')->count());
        self::assertSame($bytes, DB::table('character_command_intents')->value('intent_json'));

        try {
            $client->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Brannoc');
            self::fail('A reused idempotency key must not bind a different command.');
        } catch (CharacterCommandConflict) {
        }

        $next = $client->requestCreate('submit-2', self::ACCOUNT, self::WORLD, 'Aldric');
        self::assertNotSame($first->operationId, $next->operationId);
        self::assertSame(2, DB::table('character_command_intents')->count());

        // After a terminal receipt the same key still answers with that operation's outcome.
        $this->publish($this->rejected(CharacterCommandWire::CREATE, $first->operationId, 'CHAR_CMD_NAME_UNAVAILABLE'))->assertOk();
        $afterReceipt = $client->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        self::assertSame($first->operationId, $afterReceipt->operationId);
        self::assertSame(CharacterCommandState::Rejected, $afterReceipt->state);
        self::assertSame('CHAR_CMD_NAME_UNAVAILABLE', $afterReceipt->resultCode);
    }

    public function test_timeout_never_fails_the_intent_it_stays_pending_until_the_game_receipt(): void
    {
        $this->enable();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');

        Carbon::setTestNow('2026-10-11T12:05:01Z');
        $late = $this->client()->status($status->operationId);
        self::assertNotNull($late);
        self::assertSame(CharacterCommandState::Pending, $late->state);
        self::assertNull($late->resultCode);
        self::assertTrue($late->awaitingPastExpiry(Carbon::now()->getTimestamp()));

        $list = $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest());
        self::assertStringContainsString($status->operationId, (string) $list->getContent());
        $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::CREATE, $status->operationId))->assertOk();

        $receipt = $this->rejected(CharacterCommandWire::CREATE, $status->operationId, 'CHAR_CMD_INTENT_EXPIRED');
        $accepted = $this->publish($receipt);
        $accepted->assertOk();
        self::assertSame(CharacterCommandWire::ACCEPTED_RESPONSE, $accepted->getContent());

        $final = $this->client()->status($status->operationId);
        self::assertNotNull($final);
        self::assertSame(CharacterCommandState::Rejected, $final->state);
        self::assertSame('CHAR_CMD_INTENT_EXPIRED', $final->resultCode);
        self::assertFalse($final->awaitingPastExpiry(Carbon::now()->getTimestamp()));
        self::assertSame('{"contract_version":1,"pending":[]}', $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->getContent());
    }

    public function test_receipt_replay_is_accepted_once_and_different_bytes_conflict(): void
    {
        $this->enable();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $receipt = $this->committedCreate($status->operationId, self::ACCOUNT, '1790000002');

        $this->publish($receipt)->assertOk();
        $replay = $this->publish($receipt);
        $replay->assertOk();
        self::assertSame(CharacterCommandWire::ACCEPTED_RESPONSE, $replay->getContent());
        self::assertSame(1, DB::table('character_command_receipts')->count());

        $different = $this->publish($this->committedCreate($status->operationId, self::ACCOUNT, '1790000003'));
        $different->assertStatus(409);
        self::assertSame('', $different->getContent());
        self::assertSame(1, DB::table('character_command_receipts')->count());
        self::assertSame($receipt, DB::table('character_command_receipts')->value('receipt_json'));
        self::assertSame('00000000000000000003', DB::table('character_command_receipts')->value('recovery_generation'));

        $final = $this->client()->status($status->operationId);
        self::assertNotNull($final);
        self::assertSame(CharacterCommandState::Committed, $final->state);
        self::assertSame('CHAR_CMD_OK', $final->resultCode);
        self::assertSame(self::CHARACTER, $final->receipt['character_id'] ?? null);
    }

    public function test_receipts_are_validated_against_the_stored_intent(): void
    {
        $this->enable();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $unknown = '0192a000-0000-7000-8000-0000000000ff';

        $this->publish($this->rejected(CharacterCommandWire::CREATE, $unknown, 'CHAR_CMD_NAME_UNAVAILABLE'))->assertStatus(404);
        $this->publish('{')->assertStatus(400);
        $this->publish('')->assertStatus(400);
        $this->publish(str_replace(self::SOURCE, 'oteryn:character-authority:other', $this->rejected(CharacterCommandWire::CREATE, $status->operationId, 'CHAR_CMD_NAME_UNAVAILABLE')))->assertStatus(401);
        $this->publish($this->rejected(CharacterCommandWire::CREATE, $status->operationId, 'CHAR_CMD_OWNER_MISMATCH'))->assertStatus(400);
        $this->publish($this->rejected(CharacterCommandWire::TRANSFER, $status->operationId, 'CHAR_CMD_INTENT_EXPIRED'))->assertStatus(409);
        $this->publish($this->committedCreate($status->operationId, self::OTHER_ACCOUNT, '1790000002'))->assertStatus(409);
        $this->publish(str_replace('"Aldric"', '"Mallory"', $this->committedCreate($status->operationId, self::ACCOUNT, '1790000002')))->assertStatus(409);
        $rejected = $this->rejected(CharacterCommandWire::CREATE, $status->operationId, 'CHAR_CMD_NAME_UNAVAILABLE');
        $this->publish($rejected, ['CONTENT_TYPE' => 'text/plain'])->assertStatus(400);
        $this->publish($rejected, ['CONTENT_LENGTH' => '99999999'])->assertStatus(413);
        $this->publish($rejected, ['HTTP_TRANSFER_ENCODING' => 'chunked'])->assertStatus(400);
        $this->publish($rejected, ['HTTP_CONTENT_ENCODING' => 'gzip'])->assertStatus(400);

        self::assertSame(0, DB::table('character_command_receipts')->count());
        self::assertSame(CharacterCommandState::Pending, $this->client()->status($status->operationId)?->state);
    }

    public function test_transfer_intent_and_committed_receipt(): void
    {
        $this->enable();
        $client = $this->client();

        try {
            $client->requestTransfer('saga-1', self::CHARACTER, self::ACCOUNT, self::ACCOUNT, CharacterTransferPurpose::Settlement);
            self::fail('A transfer to the same account must be refused before storage.');
        } catch (InvalidArgumentException) {
        }

        $status = $client->requestTransfer('saga-1', self::CHARACTER, self::ACCOUNT, self::OTHER_ACCOUNT, CharacterTransferPurpose::Settlement);
        $read = $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::TRANSFER, $status->operationId));
        $read->assertOk();
        $intent = json_decode((string) $read->getContent(), true, 2, JSON_THROW_ON_ERROR);
        self::assertIsArray($intent);
        self::assertSame([
            'contract_version', 'command', 'variant', 'issuer_authority', 'issuer_decision_id', 'source_revision', 'operation_id',
            'character_id', 'expected_from_account_id', 'to_account_id', 'purpose', 'issued_at_source', 'expires_at_source', 'audience',
        ], array_keys($intent));
        self::assertSame('PLATFORM_BAZAAR_TRANSFER', $intent['variant']);
        self::assertSame('BAZAAR_SETTLEMENT', $intent['purpose']);

        $swapped = $this->committedTransfer($status->operationId, self::OTHER_ACCOUNT, self::ACCOUNT);
        $this->publish($swapped)->assertStatus(409);
        $this->publish($this->committedTransfer($status->operationId, self::ACCOUNT, self::OTHER_ACCOUNT))->assertOk();

        $final = $client->status($status->operationId);
        self::assertNotNull($final);
        self::assertSame(CharacterCommandState::Committed, $final->state);
        self::assertSame(self::OTHER_ACCOUNT, $final->receipt['to_account_id'] ?? null);
    }

    public function test_read_refuses_another_command_and_another_issuer(): void
    {
        $this->enable();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');

        $mismatch = $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::TRANSFER, $status->operationId));
        $mismatch->assertStatus(409);
        self::assertSame('', $mismatch->getContent());
        $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::CREATE, $status->operationId, 'OTHER_ISSUER'))->assertStatus(404);
        $this->exchange(CharacterCommandWire::READ_PATH, $this->readRequest(CharacterCommandWire::CREATE, '0192a000-0000-7000-8000-0000000000ff'))->assertStatus(404);
        $this->exchange(CharacterCommandWire::READ_PATH, str_replace(self::SOURCE, 'oteryn:other', $this->readRequest(CharacterCommandWire::CREATE, $status->operationId)))->assertStatus(401);
    }

    public function test_pending_list_pages_strictly_above_the_continuation_key(): void
    {
        $this->enable();
        $ids = [];
        foreach (['a', 'b', 'c'] as $key) {
            $ids[] = $this->client()->requestCreate('submit-'.$key, self::ACCOUNT, self::WORLD, 'Aldric')->operationId;
        }

        $page = json_decode((string) $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest('0', self::NIL, 2))->getContent(), true, 4, JSON_THROW_ON_ERROR);
        self::assertIsArray($page);
        $pending = $page['pending'] ?? null;
        self::assertIsArray($pending);
        self::assertSame([$ids[0], $ids[1]], array_column($pending, 'operation_id'));

        $rest = json_decode((string) $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest('2', $ids[1], 2))->getContent(), true, 4, JSON_THROW_ON_ERROR);
        self::assertIsArray($rest);
        self::assertSame([['command' => 'CreateCharacter', 'operation_id' => $ids[2], 'source_revision' => '3']], $rest['pending']);

        self::assertSame('{"contract_version":1,"pending":[]}', $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest('18446744073709551615', self::NIL, 32))->getContent());
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest('0', self::NIL, 33))->assertStatus(400);
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest('01', self::NIL, 2))->assertStatus(400);
    }

    public function test_backlog_bound_refuses_new_intents_until_a_receipt_arrives(): void
    {
        $this->enable(['character-authority.commands.max_pending_intents' => 2]);
        $client = $this->client();
        $first = $client->requestCreate('submit-a', self::ACCOUNT, self::WORLD, 'Aldric');
        $client->requestCreate('submit-b', self::ACCOUNT, self::WORLD, 'Aldric');

        try {
            $client->requestCreate('submit-c', self::ACCOUNT, self::WORLD, 'Aldric');
            self::fail('The PENDING backlog bound must refuse a new intent.');
        } catch (CharacterCommandBacklogFull) {
        }
        self::assertSame(2, DB::table('character_command_intents')->count());

        // An exact retry of a stored intent is still answered while the backlog is full.
        self::assertSame($first->operationId, $client->requestCreate('submit-a', self::ACCOUNT, self::WORLD, 'Aldric')->operationId);

        $this->publish($this->rejected(CharacterCommandWire::CREATE, $first->operationId, 'CHAR_CMD_NAME_UNAVAILABLE'))->assertOk();
        self::assertSame(CharacterCommandState::Pending, $client->requestCreate('submit-c', self::ACCOUNT, self::WORLD, 'Aldric')->state);
    }

    public function test_peer_guard_requires_the_dedicated_tls13_identity_and_rate_limits(): void
    {
        $this->enable(['character-authority.commands.requests_per_minute' => 2]);

        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest(), ['SSL_CLIENT_S_DN' => 'CN=someone-else'])->assertStatus(401);
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest(), ['SSL_CLIENT_VERIFY' => 'NONE'])->assertStatus(401);
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest(), ['SSL_PROTOCOL' => 'TLSv1.2'])->assertStatus(401);

        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->assertOk();
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->assertOk();
        $limited = $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest());
        $limited->assertStatus(429);
        self::assertNotNull($limited->headers->get('Retry-After'));

        // A subject shared with another internal purpose makes the whole surface unavailable.
        config(['game-auth.character_bootstrap_intent.mtls_client_identity' => self::PEER]);
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->assertStatus(503);
    }

    public function test_commands_stay_disabled_outside_testing_and_preproduction(): void
    {
        $this->enable();
        $this->app['env'] = 'production';

        try {
            $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->assertStatus(503);
            $this->expectException(CharacterCommandUnavailable::class);
            $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        } finally {
            $this->app['env'] = 'testing';
        }
    }

    public function test_rollback_is_refused_while_intents_exist(): void
    {
        $this->enable();
        $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $migration = require database_path('migrations/2026_10_11_120000_create_character_command_intents_and_receipts.php');
        self::assertIsObject($migration);

        try {
            (new ReflectionMethod($migration, 'down'))->invoke($migration);
            self::fail('Rollback must be refused while character command intents exist.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('rollback refused', $exception->getMessage());
        }
        self::assertTrue(DB::table('character_command_intents')->exists());
    }

    /** @param array<string, mixed> $overrides */
    private function enable(array $overrides = []): void
    {
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode([self::PEER]),
            'character-authority.commands.source_authority' => self::SOURCE,
            ...$overrides,
        ]);
    }

    private function client(): CharacterCommandClient
    {
        return app(CharacterCommandClient::class);
    }

    /**
     * @param  array<string, string>  $server
     * @return TestResponse<Response>
     */
    private function exchange(string $path, string $body, array $server = []): TestResponse
    {
        return $this->call('POST', $path, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::PEER,
            ...$server,
        ], $body);
    }

    /**
     * @param  array<string, string>  $server
     * @return TestResponse<Response>
     */
    private function publish(string $receipt, array $server = []): TestResponse
    {
        return $this->exchange(CharacterCommandWire::RECEIPT_PATH, $receipt, $server);
    }

    private function listRequest(string $after = '0', string $afterOperationId = self::NIL, int $max = 32): string
    {
        return $this->membersJson([
            'contract_version' => 1,
            'operation' => 'ListPendingCharacterCommandsV1',
            'source_authority' => self::SOURCE,
            'after_source_revision' => $after,
            'after_operation_id' => $afterOperationId,
            'max_entries' => $max,
        ]);
    }

    private function readRequest(string $command, string $operationId, string $issuer = CharacterCommandWire::ISSUER_AUTHORITY): string
    {
        return $this->membersJson([
            'contract_version' => 1,
            'operation' => 'ReadCharacterCommandIntentV1',
            'source_authority' => self::SOURCE,
            'issuer_authority' => $issuer,
            'command' => $command,
            'operation_id' => $operationId,
        ]);
    }

    private function committedCreate(string $operationId, string $accountId, string $decidedAt): string
    {
        return $this->membersJson([
            ...$this->receiptHead(CharacterCommandWire::CREATE, $operationId, 'COMMITTED', 'CHAR_CMD_OK'),
            'character_id' => self::CHARACTER,
            'account_id' => $accountId,
            'world_id' => self::WORLD,
            'name' => 'Aldric',
            'character_revision' => '1',
            'projection_epoch' => '1',
            'projection_revision' => '43',
            'recovery_generation' => '3',
            'decided_at' => $decidedAt,
        ]);
    }

    private function committedTransfer(string $operationId, string $from, string $to): string
    {
        return $this->membersJson([
            ...$this->receiptHead(CharacterCommandWire::TRANSFER, $operationId, 'COMMITTED', 'CHAR_CMD_OK'),
            'character_id' => self::CHARACTER,
            'from_account_id' => $from,
            'to_account_id' => $to,
            'character_revision' => '9',
            'projection_epoch' => '1',
            'from_projection_revision' => '44',
            'to_projection_revision' => '12',
            'recovery_generation' => '3',
            'decided_at' => '1790000002',
        ]);
    }

    private function rejected(string $command, string $operationId, string $resultCode): string
    {
        return $this->membersJson([
            ...$this->receiptHead($command, $operationId, 'REJECTED', $resultCode),
            'recovery_generation' => '3',
            'decided_at' => '1790000002',
        ]);
    }

    /** @return array<string, int|string> */
    private function receiptHead(string $command, string $operationId, string $outcome, string $resultCode): array
    {
        return [
            'contract_version' => 1,
            'operation' => 'PublishCharacterCommandReceiptV1',
            'source_authority' => self::SOURCE,
            'command' => $command,
            'operation_id' => $operationId,
            'outcome' => $outcome,
            'result_code' => $resultCode,
        ];
    }

    /** @param array<string, int|string> $members */
    private function membersJson(array $members): string
    {
        return json_encode($members, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
