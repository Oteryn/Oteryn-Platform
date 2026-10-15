<?php

namespace Tests\Feature\CharacterAuthority;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandState;
use App\CharacterAuthority\CharacterCommandWire;
use App\CharacterAuthority\Restore\GameRestoreNoticeWire;
use App\GameProjections\PublicProjectionSettings;
use ArrayObject;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Log\Events\MessageLogged;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Event;
use Illuminate\Testing\TestResponse;
use LogicException;
use ReflectionMethod;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;

final class GameRestoreReconciliationTest extends TestCase
{
    use RefreshDatabase;

    private const COMMAND_PEER = 'CN=oteryn-game-character-commands';

    private const RESTORE_PEER = 'CN=oteryn-game-restore-notice';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const CHARACTER = '01934f10-7c04-7001-805b-3b1122334401';

    private const EVENT = '01934f10-7c05-7001-805b-3b1122334401';

    private const SECOND_EVENT = '01934f10-7c06-7001-805b-3b1122334401';

    private const NIL = '00000000-0000-0000-0000-000000000000';

    protected function setUp(): void
    {
        parent::setUp();
        Carbon::setTestNow('2026-10-15T12:00:00Z');
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_restore_notice_is_disabled_by_default(): void
    {
        self::assertFalse(config('character-authority.restore.enabled'));

        $response = $this->notice($this->noticeBody(self::EVENT, '3', '4'));
        $response->assertStatus(503);
        self::assertSame('', $response->getContent());
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));
        self::assertSame(0, DB::table('character_restore_notices')->count());
    }

    public function test_notice_reopens_only_receipts_at_or_above_the_restored_generation(): void
    {
        $this->enableCommands();
        $this->enableRestore();
        $kept = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $lost = $this->client()->requestCreate('submit-2', self::ACCOUNT, self::WORLD, 'Brannoc');
        $open = $this->client()->requestCreate('submit-3', self::ACCOUNT, self::WORLD, 'Cedric');
        $this->publish($this->rejected($kept->operationId, '2'))->assertOk();
        $lostReceipt = $this->committedCreate($lost->operationId, '3', 'Brannoc');
        $this->publish($lostReceipt)->assertOk();

        $response = $this->notice($this->noticeBody(self::EVENT, '3', '4'));
        $response->assertOk();
        self::assertSame(
            '{"contract_version":1,"restore_event_id":"'.self::EVENT.'","status":"RECONCILING","reopened":1,"outstanding":1}',
            $response->getContent(),
        );
        self::assertStringContainsString('no-store', (string) $response->headers->get('Cache-Control'));

        // Generation 2 stays decided; generation 3 is reopened and its receipt kept as history.
        self::assertSame(CharacterCommandState::Rejected, $this->client()->status($kept->operationId)?->state);
        self::assertSame(CharacterCommandState::Pending, $this->client()->status($lost->operationId)?->state);
        self::assertSame(CharacterCommandState::Pending, $this->client()->status($open->operationId)?->state);
        $history = DB::table('character_command_receipts')->where('operation_id', $lost->operationId)->first();
        self::assertNotNull($history);
        self::assertNotNull($history->superseded_at);
        self::assertSame($lostReceipt, $history->receipt_json);
        self::assertNull(DB::table('character_command_receipts')->where('operation_id', $kept->operationId)->value('superseded_at'));

        // Game sees the reopened operation again under its unchanged identity and intent bytes.
        $list = (string) $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->getContent();
        self::assertStringContainsString($lost->operationId, $list);
        self::assertStringContainsString($open->operationId, $list);
        self::assertStringNotContainsString($kept->operationId, $list);

        // An identical repeat reopens nothing more and answers the same status.
        $repeat = $this->notice($this->noticeBody(self::EVENT, '3', '4'));
        self::assertSame($response->getContent(), $repeat->getContent());
        self::assertSame(1, DB::table('character_restore_reopens')->count());
        self::assertNull(DB::table('character_restore_notices')->value('acknowledged_at'));

        // Exactly one new receipt per operation, from the restored Game; then the acknowledgement.
        $this->publish($this->rejected($lost->operationId, '4', 'CHAR_CMD_NAME_UNAVAILABLE'))->assertOk();
        self::assertSame(2, DB::table('character_command_receipts')->where('operation_id', $lost->operationId)->count());
        self::assertSame(CharacterCommandState::Rejected, $this->client()->status($lost->operationId)?->state);
        $acknowledged = $this->notice($this->noticeBody(self::EVENT, '3', '4'));
        self::assertSame(
            '{"contract_version":1,"restore_event_id":"'.self::EVENT.'","status":"ACKNOWLEDGED","reopened":1,"outstanding":0}',
            $acknowledged->getContent(),
        );
        self::assertNotNull(DB::table('character_restore_notices')->value('acknowledged_at'));
        self::assertNotNull(DB::table('character_restore_reopens')->value('new_receipt_id'));

        // The superseded receipt cannot come back as a republication of the decided operation.
        $this->publish($lostReceipt)->assertStatus(409);
    }

    public function test_committed_outcome_reversed_by_the_restored_game_raises_the_compensation_alarm(): void
    {
        $this->enableCommands();
        $this->enableRestore();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $this->publish($this->committedCreate($status->operationId, '5'))->assertOk();
        $this->notice($this->noticeBody(self::EVENT, '5', '6'))->assertOk();

        /** @var ArrayObject<int, mixed> $reversed */
        $reversed = new ArrayObject;
        Event::listen(MessageLogged::class, static function (MessageLogged $logged) use ($reversed): void {
            if ($logged->level === 'error' && $logged->message === 'character_authority.restore_outcome_reversed') {
                $reversed->append($logged->context['operation_id'] ?? null);
            }
        });
        $this->publish($this->rejected($status->operationId, '6', 'CHAR_CMD_NAME_UNAVAILABLE'))->assertOk();

        self::assertSame([$status->operationId], $reversed->getArrayCopy());
    }

    public function test_restored_game_may_republish_the_same_decision_as_the_new_receipt(): void
    {
        $this->enableCommands();
        $this->enableRestore();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $receipt = $this->committedCreate($status->operationId, '3');
        $this->publish($receipt)->assertOk();
        $this->notice($this->noticeBody(self::EVENT, '3', '4'))->assertOk();

        // The decision predates the snapshot within generation 3, so Game still holds it.
        $this->publish($receipt)->assertOk();
        self::assertSame(2, DB::table('character_command_receipts')->count());
        self::assertSame(CharacterCommandState::Committed, $this->client()->status($status->operationId)?->state);
        self::assertStringContainsString('"ACKNOWLEDGED"', (string) $this->notice($this->noticeBody(self::EVENT, '3', '4'))->getContent());
    }

    public function test_notice_without_affected_operations_is_acknowledged_at_once_even_with_commands_disabled(): void
    {
        $this->enableRestore();
        self::assertFalse(config('character-authority.commands.enabled'));

        $response = $this->notice($this->noticeBody(self::EVENT, '3', '4'));
        $response->assertOk();
        self::assertSame(
            '{"contract_version":1,"restore_event_id":"'.self::EVENT.'","status":"ACKNOWLEDGED","reopened":0,"outstanding":0}',
            $response->getContent(),
        );
        self::assertNotNull(DB::table('character_restore_notices')->value('acknowledged_at'));
    }

    public function test_a_later_restore_also_waits_for_operations_an_earlier_one_left_open(): void
    {
        $this->enableCommands();
        $this->enableRestore();
        $first = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $second = $this->client()->requestCreate('submit-2', self::ACCOUNT, self::WORLD, 'Brannoc');
        $this->publish($this->committedCreate($first->operationId, '3'))->assertOk();
        $this->publish($this->rejected($second->operationId, '2'))->assertOk();

        $this->notice($this->noticeBody(self::EVENT, '3', '4'))->assertOk();
        // Game is restored again, further back, before the first notice's operation was answered.
        $later = $this->notice($this->noticeBody(self::SECOND_EVENT, '2', '5'));
        self::assertSame(
            '{"contract_version":1,"restore_event_id":"'.self::SECOND_EVENT.'","status":"RECONCILING","reopened":2,"outstanding":2}',
            $later->getContent(),
        );

        $this->publish($this->rejected($first->operationId, '5', 'CHAR_CMD_NAME_UNAVAILABLE'))->assertOk();
        $this->publish($this->rejected($second->operationId, '5', 'CHAR_CMD_NAME_UNAVAILABLE'))->assertOk();
        self::assertStringContainsString('"ACKNOWLEDGED"', (string) $this->notice($this->noticeBody(self::SECOND_EVENT, '2', '5'))->getContent());
        self::assertStringContainsString('"ACKNOWLEDGED"', (string) $this->notice($this->noticeBody(self::EVENT, '3', '4'))->getContent());
    }

    public function test_conflicting_stale_and_malformed_notices_are_refused(): void
    {
        $this->enableRestore();
        $this->notice($this->noticeBody(self::EVENT, '3', '4'))->assertOk();

        $this->notice($this->noticeBody(self::EVENT, '3', '4', 'other-backup'))->assertStatus(409);
        $this->notice($this->noticeBody(self::SECOND_EVENT, '2', '4'))->assertStatus(409);
        self::assertSame(1, DB::table('character_restore_notices')->count());

        $this->notice($this->noticeBody(self::SECOND_EVENT, '4', '4'))->assertStatus(400);
        $this->notice($this->noticeBody(self::SECOND_EVENT, '0', '5'))->assertStatus(400);
        $this->notice($this->noticeBody(self::SECOND_EVENT, '4', '5', 'bad label'))->assertStatus(400);
        $this->notice(substr($this->noticeBody(self::SECOND_EVENT, '4', '5'), 0, -1).',"extra":1}')->assertStatus(400);
        $this->notice($this->noticeBody(self::SECOND_EVENT, '4', '5', source: 'oteryn:character-authority:other'))->assertStatus(401);
        $this->notice(str_repeat(' ', GameRestoreNoticeWire::MAX_REQUEST_BYTES + 1), ['CONTENT_LENGTH' => (string) (GameRestoreNoticeWire::MAX_REQUEST_BYTES + 1)])->assertStatus(413);
        $this->notice(str_repeat(' ', GameRestoreNoticeWire::MAX_REQUEST_BYTES + 1))->assertStatus(400);
        self::assertFalse(DB::table('character_restore_notices')->where('restore_event_id', self::SECOND_EVENT)->exists());
    }

    public function test_peer_guard_requires_the_dedicated_tls13_identity_and_rate_limits(): void
    {
        $this->enableRestore(['character-authority.restore.requests_per_minute' => 1]);
        $body = $this->noticeBody(self::EVENT, '3', '4');

        $this->notice($body, ['SSL_CLIENT_S_DN' => self::COMMAND_PEER])->assertStatus(401);
        $this->notice($body, ['SSL_PROTOCOL' => 'TLSv1.2'])->assertStatus(401);
        $this->notice($body, ['SSL_CLIENT_VERIFY' => 'NONE'])->assertStatus(401);
        $this->notice($body)->assertOk();
        $limited = $this->notice($body);
        $limited->assertStatus(429);
        self::assertNotNull($limited->headers->get('Retry-After'));

        // An identity shared with the character-command consumer disables both surfaces.
        $this->enableCommands();
        $this->enableRestore(['character-authority.restore.identities' => json_encode([self::COMMAND_PEER])]);
        $this->notice($body, ['SSL_CLIENT_S_DN' => self::COMMAND_PEER])->assertStatus(503);
        $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->assertStatus(503);

        // So does one shared with the public-projection consumer.
        $this->enableRestore();
        config([
            'game-projections.public.enabled' => true,
            'game-projections.public.identities' => [self::RESTORE_PEER],
            'game-projections.public.source_authority' => self::SOURCE,
        ]);
        $this->notice($body)->assertStatus(503);
        self::assertNull(PublicProjectionSettings::current());
    }

    public function test_lost_platform_receipt_self_heals_from_the_game_republication(): void
    {
        $this->enableCommands();
        $status = $this->client()->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $receipt = $this->committedCreate($status->operationId, '3');
        $this->publish($receipt)->assertOk();

        // A Platform restore to a point before the receipt arrived: the intent is PENDING again.
        DB::table('character_command_intents')->update(['state' => 'PENDING', 'current_receipt_id' => null]);
        DB::table('character_command_receipts')->delete();

        self::assertStringContainsString($status->operationId, (string) $this->exchange(CharacterCommandWire::LIST_PATH, $this->listRequest())->getContent());
        $this->publish($receipt)->assertOk();
        self::assertSame(CharacterCommandState::Committed, $this->client()->status($status->operationId)?->state);
        self::assertSame(1, DB::table('character_command_receipts')->count());
    }

    public function test_rollback_is_refused_while_restore_notices_exist(): void
    {
        $this->enableRestore();
        $this->notice($this->noticeBody(self::EVENT, '3', '4'))->assertOk();
        $migration = require database_path('migrations/2026_10_15_120000_create_character_restore_notices.php');
        self::assertIsObject($migration);

        try {
            (new ReflectionMethod($migration, 'down'))->invoke($migration);
            self::fail('Rollback must be refused while Game restore notices exist.');
        } catch (LogicException $exception) {
            self::assertStringContainsString('rollback refused', $exception->getMessage());
        }
        self::assertTrue(DB::table('character_restore_notices')->exists());
    }

    private function enableCommands(): void
    {
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode([self::COMMAND_PEER]),
            'character-authority.commands.source_authority' => self::SOURCE,
        ]);
    }

    /** @param array<string, mixed> $overrides */
    private function enableRestore(array $overrides = []): void
    {
        config([
            'character-authority.restore.enabled' => true,
            'character-authority.restore.identities' => json_encode([self::RESTORE_PEER]),
            'character-authority.restore.source_authority' => self::SOURCE,
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
    private function notice(string $body, array $server = []): TestResponse
    {
        return $this->call('POST', GameRestoreNoticeWire::PATH, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::RESTORE_PEER,
            ...$server,
        ], $body);
    }

    /** @return TestResponse<Response> */
    private function exchange(string $path, string $body): TestResponse
    {
        return $this->call('POST', $path, [], [], [], [
            'CONTENT_TYPE' => 'application/json',
            'SSL_CLIENT_VERIFY' => 'SUCCESS',
            'SSL_PROTOCOL' => 'TLSv1.3',
            'SSL_CLIENT_S_DN' => self::COMMAND_PEER,
        ], $body);
    }

    /** @return TestResponse<Response> */
    private function publish(string $receipt): TestResponse
    {
        return $this->exchange(CharacterCommandWire::RECEIPT_PATH, $receipt);
    }

    private function noticeBody(string $eventId, string $restored, string $new, string $label = 'nas-2026-10-15T11:00Z', string $source = self::SOURCE): string
    {
        return $this->membersJson([
            'contract_version' => 1,
            'operation' => 'PublishGameRestoreNoticeV1',
            'source_authority' => $source,
            'restore_event_id' => $eventId,
            'restored_generation' => $restored,
            'new_recovery_generation' => $new,
            'restore_point' => '1791976000',
            'backup_label' => $label,
        ]);
    }

    private function listRequest(): string
    {
        return $this->membersJson([
            'contract_version' => 1,
            'operation' => 'ListPendingCharacterCommandsV1',
            'source_authority' => self::SOURCE,
            'after_source_revision' => '0',
            'after_operation_id' => self::NIL,
            'max_entries' => 32,
        ]);
    }

    private function committedCreate(string $operationId, string $generation, string $name = 'Aldric'): string
    {
        return $this->membersJson([
            ...$this->receiptHead($operationId, 'COMMITTED', 'CHAR_CMD_OK'),
            'character_id' => self::CHARACTER,
            'account_id' => self::ACCOUNT,
            'world_id' => self::WORLD,
            'name' => $name,
            'character_revision' => '1',
            'projection_epoch' => '1',
            'projection_revision' => '43',
            'recovery_generation' => $generation,
            'decided_at' => '1790000002',
        ]);
    }

    private function rejected(string $operationId, string $generation, string $resultCode = 'CHAR_CMD_NAME_UNAVAILABLE'): string
    {
        return $this->membersJson([
            ...$this->receiptHead($operationId, 'REJECTED', $resultCode),
            'recovery_generation' => $generation,
            'decided_at' => '1790000002',
        ]);
    }

    /** @return array<string, int|string> */
    private function receiptHead(string $operationId, string $outcome, string $resultCode): array
    {
        return [
            'contract_version' => 1,
            'operation' => 'PublishCharacterCommandReceiptV1',
            'source_authority' => self::SOURCE,
            'command' => CharacterCommandWire::CREATE,
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
