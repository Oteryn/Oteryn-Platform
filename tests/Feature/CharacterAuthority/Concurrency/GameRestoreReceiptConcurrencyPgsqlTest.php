<?php

namespace Tests\Feature\CharacterAuthority\Concurrency;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandExchange;
use App\CharacterAuthority\CharacterCommandIntentRow;
use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandStoredReceipt;
use App\CharacterAuthority\CharacterCommandWire;
use App\CharacterAuthority\Restore\GameRestoreNoticeSettings;
use App\CharacterAuthority\Restore\GameRestoreReconciler;
use Illuminate\Support\Facades\DB;
use RuntimeException;
use Tests\Support\Concurrency\ForkedWorkers;
use Tests\Support\Concurrency\PgsqlConcurrencyDatabase;
use Tests\TestCase;

/**
 * A receipt that is still committing when a Game restore notice arrives is never missed: the
 * notice waits on the restore lock, then reopens the receipt it would otherwise not have seen. In a
 * chain of restores, an operation is either carried open or reopened from its new receipt, never
 * dropped, so the newest notice is not acknowledged early.
 * Two real PHP processes against one PostgreSQL database. Skipped unless PGSQL_INTEGRATION_HOST
 * is configured.
 */
final class GameRestoreReceiptConcurrencyPgsqlTest extends TestCase
{
    private const COMMAND_PEER = 'CN=oteryn-game-character-commands';

    private const RESTORE_PEER = 'CN=oteryn-game-restore-notice';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const EVENT = '01934f10-7c05-7001-805b-3b1122334401';

    private const SECOND_EVENT = '01934f10-7c06-7001-805b-3b1122334401';

    private const THIRD_EVENT = '01934f10-7c07-7001-805b-3b1122334401';

    private PgsqlConcurrencyDatabase $database;

    protected function setUp(): void
    {
        parent::setUp();

        $reason = PgsqlConcurrencyDatabase::unavailableReason();
        if ($reason !== null) {
            $this->markTestSkipped($reason);
        }

        $this->database = new PgsqlConcurrencyDatabase;
        $this->database->create('oteryn_restore_receipt_concurrency_test');
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode([self::COMMAND_PEER]),
            'character-authority.commands.source_authority' => self::SOURCE,
            'character-authority.restore.enabled' => true,
            'character-authority.restore.identities' => json_encode([self::RESTORE_PEER]),
            'character-authority.restore.source_authority' => self::SOURCE,
        ]);
    }

    protected function tearDown(): void
    {
        if (isset($this->database)) {
            $this->database->drop();
        }

        parent::tearDown();
    }

    public function test_a_receipt_committing_during_a_restore_notice_is_reopened(): void
    {
        $intent = app(CharacterCommandClient::class)->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $receipt = $this->canonical([
            'contract_version' => 1,
            'operation' => 'PublishCharacterCommandReceiptV1',
            'source_authority' => self::SOURCE,
            'command' => CharacterCommandWire::CREATE,
            'operation_id' => $intent->operationId,
            'outcome' => 'REJECTED',
            'result_code' => 'CHAR_CMD_NAME_UNAVAILABLE',
            'recovery_generation' => '3',
            'decided_at' => '1790000002',
        ]);
        $notice = $this->canonical([
            'contract_version' => 1,
            'operation' => 'PublishGameRestoreNoticeV1',
            'source_authority' => self::SOURCE,
            'restore_event_id' => self::EVENT,
            'restored_generation' => '3',
            'new_recovery_generation' => '4',
            'restore_point' => '1791976000',
            'backup_label' => 'nas-2026-10-15T11:00Z',
        ]);

        $results = ForkedWorkers::run(2, 'pgsql', function (int $worker) use ($receipt, $notice): array {
            if ($worker === 0) {
                // The receipt stays uncommitted, holding its locks, while the notice arrives.
                DB::transaction(function () use ($receipt): void {
                    app(CharacterCommandExchange::class)->receipt(
                        CharacterCommandSettings::current() ?? throw new RuntimeException('Commands are not configured.'),
                        $receipt,
                    );
                    usleep(1_500_000);
                });

                return ['outcome' => 'receipt_stored'];
            }

            usleep(500_000);

            return ['outcome' => 'notice_answered', 'body' => app(GameRestoreReconciler::class)->notice(
                GameRestoreNoticeSettings::current() ?? throw new RuntimeException('Restore notices are not configured.'),
                $notice,
            )];
        });

        $evidence = json_encode($results, JSON_THROW_ON_ERROR);
        self::assertSame('receipt_stored', $results[0]['outcome'] ?? null, $evidence);
        self::assertSame('notice_answered', $results[1]['outcome'] ?? null, $evidence);
        self::assertSame(
            '{"contract_version":1,"restore_event_id":"'.self::EVENT.'","status":"RECONCILING","reopened":1,"outstanding":1}',
            $results[1]['body'] ?? null,
            $evidence,
        );

        $row = DB::table(CharacterCommandIntentRow::TABLE)->where('operation_id', $intent->operationId)->first();
        self::assertNotNull($row);
        self::assertSame(CharacterCommandIntentRow::PENDING, $row->state ?? null);
        self::assertNull($row->current_receipt_id ?? null);
        self::assertSame(1, DB::table(GameRestoreReconciler::REOPENS)->whereNull('new_receipt_id')->count());
        self::assertSame(1, DB::table(CharacterCommandStoredReceipt::TABLE)->whereNotNull('superseded_at')->count());
    }

    public function test_a_carried_reopen_answered_while_a_chained_notice_waits_is_reopened_not_dropped(): void
    {
        $intent = app(CharacterCommandClient::class)->requestCreate('submit-1', self::ACCOUNT, self::WORLD, 'Aldric');
        $exchange = app(CharacterCommandExchange::class);
        $reconciler = app(GameRestoreReconciler::class);
        $commands = CharacterCommandSettings::current() ?? throw new RuntimeException('Commands are not configured.');
        $restore = GameRestoreNoticeSettings::current() ?? throw new RuntimeException('Restore notices are not configured.');
        // H3→4 reopens the generation-3 receipt and H2→5 carries the still open operation.
        $exchange->receipt($commands, $this->rejectedReceipt($intent->operationId, '3'));
        $reconciler->notice($restore, $this->noticeJson(self::EVENT, '3', '4'));
        $reconciler->notice($restore, $this->noticeJson(self::SECOND_EVENT, '2', '5'));

        $receipt = $this->rejectedReceipt($intent->operationId, '5');
        $notice = $this->noticeJson(self::THIRD_EVENT, '5', '6');
        $results = ForkedWorkers::run(2, 'pgsql', function (int $worker) use ($receipt, $notice): array {
            if ($worker === 0) {
                // The generation-5 answer stays uncommitted while the H5→6 notice arrives.
                DB::transaction(function () use ($receipt): void {
                    app(CharacterCommandExchange::class)->receipt(
                        CharacterCommandSettings::current() ?? throw new RuntimeException('Commands are not configured.'),
                        $receipt,
                    );
                    usleep(1_500_000);
                });

                return ['outcome' => 'receipt_stored'];
            }

            usleep(500_000);

            return ['outcome' => 'notice_answered', 'body' => app(GameRestoreReconciler::class)->notice(
                GameRestoreNoticeSettings::current() ?? throw new RuntimeException('Restore notices are not configured.'),
                $notice,
            )];
        });

        $evidence = json_encode($results, JSON_THROW_ON_ERROR);
        self::assertSame('receipt_stored', $results[0]['outcome'] ?? null, $evidence);
        self::assertSame(
            '{"contract_version":1,"restore_event_id":"'.self::THIRD_EVENT.'","status":"RECONCILING","reopened":1,"outstanding":1}',
            $results[1]['body'] ?? null,
            $evidence,
        );

        // The answer closed both earlier notices; the newest reopened it and waits for generation 6.
        $row = DB::table(CharacterCommandIntentRow::TABLE)->where('operation_id', $intent->operationId)->first();
        self::assertNotNull($row);
        self::assertSame(CharacterCommandIntentRow::PENDING, $row->state ?? null);
        $open = DB::table(GameRestoreReconciler::REOPENS.' as reopen')
            ->join(GameRestoreReconciler::NOTICES.' as notice', 'notice.id', '=', 'reopen.notice_id')
            ->whereNull('reopen.new_receipt_id')
            ->pluck('notice.restore_event_id')
            ->all();
        self::assertSame([self::THIRD_EVENT], $open);
        self::assertSame(2, DB::table(CharacterCommandStoredReceipt::TABLE)->whereNotNull('superseded_at')->count());

        $exchange->receipt($commands, $this->rejectedReceipt($intent->operationId, '6'));
        foreach ([[self::THIRD_EVENT, '5', '6'], [self::SECOND_EVENT, '2', '5'], [self::EVENT, '3', '4']] as [$event, $restored, $new]) {
            self::assertStringContainsString('"ACKNOWLEDGED"', $reconciler->notice($restore, $this->noticeJson($event, $restored, $new)));
        }
    }

    private function rejectedReceipt(string $operationId, string $generation): string
    {
        return $this->canonical([
            'contract_version' => 1,
            'operation' => 'PublishCharacterCommandReceiptV1',
            'source_authority' => self::SOURCE,
            'command' => CharacterCommandWire::CREATE,
            'operation_id' => $operationId,
            'outcome' => 'REJECTED',
            'result_code' => 'CHAR_CMD_NAME_UNAVAILABLE',
            'recovery_generation' => $generation,
            'decided_at' => '1790000002',
        ]);
    }

    private function noticeJson(string $eventId, string $restored, string $new): string
    {
        return $this->canonical([
            'contract_version' => 1,
            'operation' => 'PublishGameRestoreNoticeV1',
            'source_authority' => self::SOURCE,
            'restore_event_id' => $eventId,
            'restored_generation' => $restored,
            'new_recovery_generation' => $new,
            'restore_point' => '1791976000',
            'backup_label' => 'nas-2026-10-15T11:00Z',
        ]);
    }

    /** @param array<string, int|string> $members */
    private function canonical(array $members): string
    {
        return json_encode($members, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    }
}
