<?php

namespace Tests\Feature\CharacterAuthority\Concurrency;

use App\CharacterAuthority\CharacterCommandClient;
use App\CharacterAuthority\CharacterCommandExchange;
use App\CharacterAuthority\CharacterCommandSettings;
use App\CharacterAuthority\CharacterCommandUnavailable;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use LogicException;
use Mockery;
use Tests\Support\Concurrency\ForkedWorkers;
use Tests\Support\Concurrency\PgsqlConcurrencyDatabase;
use Tests\TestCase;

/**
 * Game U-CC9 on PostgreSQL: concurrent issuers, each committing inside an outer transaction as
 * CreateCharacter does, never commit an intent behind a forward-only Game consumer cursor. A
 * consumer that never rescans must therefore list every intent exactly once, in key order.
 * Skipped unless PGSQL_INTEGRATION_HOST is configured.
 */
final class CharacterCommandIssuanceOrderPgsqlTest extends TestCase
{
    private const ISSUERS = 7;

    /** 7 x 5 = 35 unreceipted intents, below the default pending backlog of 40 (gate U-CC9). */
    private const INTENTS_PER_ISSUER = 5;

    private const ACCOUNT = '0190f2a1-3b4c-7d5e-8f60-718293a4b5c6';

    private const WORLD = '01934f10-7c02-7001-805b-3b1122334401';

    private const SOURCE = 'oteryn:character-authority:primary';

    private const NIL = '00000000-0000-0000-0000-000000000000';

    private PgsqlConcurrencyDatabase $database;

    protected function setUp(): void
    {
        parent::setUp();

        $reason = PgsqlConcurrencyDatabase::unavailableReason();
        if ($reason !== null) {
            $this->markTestSkipped($reason);
        }

        $this->database = new PgsqlConcurrencyDatabase;
        $this->database->create('oteryn_char_cmd_order_concurrency_test');
        config([
            'character-authority.commands.enabled' => true,
            'character-authority.commands.identities' => json_encode(['CN=oteryn-game-character-commands']),
            'character-authority.commands.source_authority' => self::SOURCE,
        ]);
    }

    protected function tearDown(): void
    {
        if (isset($this->database)) {
            $this->database->drop();
        }

        parent::tearDown();
    }

    public function test_a_forward_only_consumer_lists_every_concurrently_issued_intent(): void
    {
        $total = self::ISSUERS * self::INTENTS_PER_ISSUER;
        // The consumer never receipts, so the whole race must fit the pending backlog ceiling.
        $settings = CharacterCommandSettings::current() ?? throw new LogicException('Fixture command settings are unavailable.');
        self::assertLessThanOrEqual($settings->maxPendingIntents, $total);

        $results = ForkedWorkers::run(self::ISSUERS + 1, 'pgsql', fn (int $worker): array => $worker === self::ISSUERS
            ? $this->consume($total)
            : $this->issue($worker));

        $consumer = array_pop($results);
        foreach ($results as $result) {
            self::assertSame('issued', $result['outcome'] ?? null, json_encode($results, JSON_THROW_ON_ERROR));
        }
        self::assertSame('consumed', $consumer['outcome'] ?? null, json_encode($consumer, JSON_THROW_ON_ERROR));

        $listed = $consumer['revisions'] ?? null;
        self::assertIsArray($listed);
        $stored = [];
        foreach (DB::table('character_command_intents')->orderBy('id')->pluck('id') as $id) {
            self::assertIsInt($id);
            $stored[] = (string) $id;
        }
        self::assertCount($total, $stored);
        // Every stored intent was listed by a cursor that never moved back, so none committed behind it.
        self::assertSame($stored, $listed);
    }

    public function test_an_intent_drawn_below_a_stored_key_is_refused(): void
    {
        $client = app(CharacterCommandClient::class);
        $client->requestCreate('submit-a', self::ACCOUNT, self::WORLD, 'Aldric');
        DB::statement("select setval(pg_get_serial_sequence('character_command_intents', 'id'), 1000)");
        $client->requestCreate('submit-b', self::ACCOUNT, self::WORLD, 'Aldric');
        // A sequence left behind its table, as a restore that skipped setval leaves it.
        DB::statement("select setval(pg_get_serial_sequence('character_command_intents', 'id'), 10)");
        $logger = Log::spy();

        try {
            $client->requestCreate('submit-d', self::ACCOUNT, self::WORLD, 'Aldric');
            self::fail('An intent below a stored key would commit behind the Game consumer cursor.');
        } catch (CharacterCommandUnavailable) {
        }

        self::assertSame([1, 1001], DB::table('character_command_intents')->orderBy('id')->pluck('id')->all());
        $logger->shouldHaveReceived('error')->once()->with('character_authority.command_source_revision_behind', Mockery::type('array'));
    }

    /** @return array<string, mixed> */
    private function issue(int $worker): array
    {
        $client = app(CharacterCommandClient::class);
        for ($i = 0; $i < self::INTENTS_PER_ISSUER; $i++) {
            // The outer transaction keeps the issuer lock until it commits, as CreateCharacter's does.
            DB::transaction(function () use ($client, $worker, $i): void {
                $client->requestCreate("race-{$worker}-{$i}", self::ACCOUNT, self::WORLD, 'Aldric');
                usleep(random_int(0, 1_500));
            });
        }

        return ['outcome' => 'issued'];
    }

    /** @return array<string, mixed> */
    private function consume(int $total): array
    {
        $settings = CharacterCommandSettings::current();
        if ($settings === null) {
            return ['outcome' => 'disabled'];
        }
        $exchange = app(CharacterCommandExchange::class);
        $after = '0';
        $afterOperationId = self::NIL;
        $revisions = [];
        $deadline = microtime(true) + 30;
        while (count($revisions) < $total && microtime(true) < $deadline) {
            $page = json_decode($exchange->pending($settings, $this->listRequest($after, $afterOperationId)), true, 4, JSON_THROW_ON_ERROR);
            $entries = is_array($page) && is_array($page['pending'] ?? null) ? $page['pending'] : [];
            foreach ($entries as $entry) {
                if (! is_array($entry) || ! is_string($entry['source_revision'] ?? null) || ! is_string($entry['operation_id'] ?? null)) {
                    return ['outcome' => 'malformed'];
                }
                $revisions[] = $entry['source_revision'];
                $after = $entry['source_revision'];
                $afterOperationId = $entry['operation_id'];
            }
            if ($entries === []) {
                usleep(500);
            }
        }

        return ['outcome' => count($revisions) === $total ? 'consumed' : 'missed', 'revisions' => $revisions];
    }

    private function listRequest(string $after, string $afterOperationId): string
    {
        return json_encode([
            'contract_version' => 1,
            'operation' => 'ListPendingCharacterCommandsV1',
            'source_authority' => self::SOURCE,
            'after_source_revision' => $after,
            'after_operation_id' => $afterOperationId,
            'max_entries' => 32,
        ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
    }
}
