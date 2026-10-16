<?php

namespace App\CharacterAuthority;

use App\Identity\Support\CanonicalAccountId;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use InvalidArgumentException;
use stdClass;

/**
 * Platform command client for Game Character Authority commands v1. Platform never calls Game: a
 * request stores one immutable intent, keyed by a fresh UUIDv7 `operation_id`, which the Game
 * command consumer pulls; the result is the receipt Game publishes back (Game contract §2).
 *
 * The caller's idempotency key (one per user submission or saga step) maps to exactly one intent,
 * so a double submit or saga retry gets the same `operation_id` and bytes; a new attempt after a
 * terminal receipt uses a new key. Authorization of the user or saga step is the caller's duty.
 */
final class CharacterCommandClient
{
    private const MAX_IDEMPOTENCY_KEY_BYTES = 191;

    public function requestCreate(string $idempotencyKey, string $accountId, string $targetWorldId, string $requestedName): CharacterCommandStatus
    {
        CharacterCommandWire::assertUuid7($accountId, 'account_id');
        CharacterCommandWire::assertUuid7($targetWorldId, 'target_world_id');
        CharacterCommandWire::assertRequestedName($requestedName);

        return $this->request($idempotencyKey, CharacterCommandWire::CREATE, [
            'account_id' => $accountId,
            'target_world_id' => $targetWorldId,
            'requested_name' => $requestedName,
        ]);
    }

    public function requestTransfer(string $idempotencyKey, string $characterId, string $expectedFromAccountId, string $toAccountId, CharacterTransferPurpose $purpose): CharacterCommandStatus
    {
        CharacterCommandWire::assertUuid7($characterId, 'character_id');
        CharacterCommandWire::assertUuid7($expectedFromAccountId, 'expected_from_account_id');
        CharacterCommandWire::assertUuid7($toAccountId, 'to_account_id');
        if ($expectedFromAccountId === $toAccountId) {
            throw new InvalidArgumentException('to_account_id must differ from expected_from_account_id.');
        }

        return $this->request($idempotencyKey, CharacterCommandWire::TRANSFER, [
            'character_id' => $characterId,
            'expected_from_account_id' => $expectedFromAccountId,
            'to_account_id' => $toAccountId,
            'purpose' => $purpose->value,
        ]);
    }

    public function status(string $operationId): ?CharacterCommandStatus
    {
        CharacterCommandWire::assertUuid7($operationId, 'operation_id');
        $row = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('operation_id', $operationId)
            ->first();

        return $row instanceof stdClass ? $this->statusOf(CharacterCommandIntentRow::fromRow($row)) : null;
    }

    /**
     * A `CreateCharacter` command's status, only when its intent was issued for `$accountId`. An
     * unknown, malformed or foreign operation id answers null, so a caller cannot probe others'.
     */
    public function createStatusFor(string $operationId, string $accountId): ?CharacterCommandStatus
    {
        if (preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/D', $operationId) !== 1) {
            return null;
        }
        $row = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('operation_id', $operationId)
            ->where('command', CharacterCommandWire::CREATE)
            ->whereNotNull('intent_json')
            ->first();
        if (! $row instanceof stdClass) {
            return null;
        }
        $intent = CharacterCommandIntentRow::fromRow($row);
        $owner = $intent->intent()['account_id'] ?? null;
        if (! is_string($owner) || ! hash_equals($owner, $accountId)) {
            return null;
        }

        return $this->statusOf($intent);
    }

    /** @param array<string, string> $binding */
    private function request(string $idempotencyKey, string $command, array $binding): CharacterCommandStatus
    {
        $settings = CharacterCommandSettings::current()
            ?? throw new CharacterCommandUnavailable('Character Authority commands are disabled.');
        if (strlen($idempotencyKey) > self::MAX_IDEMPOTENCY_KEY_BYTES || preg_match('/^[\x21-\x7e]+$/D', $idempotencyKey) !== 1) {
            throw new InvalidArgumentException('Idempotency key must be 1..191 printable ASCII bytes.');
        }
        $keySha = hash('sha256', $idempotencyKey);
        $bindingSha = hash('sha256', json_encode([$command, $binding], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES));

        try {
            return $this->issue($settings, $keySha, $bindingSha, $command, $binding);
        } catch (UniqueConstraintViolationException) {
            // A concurrent request with the same key committed first: answer with its intent.
            return $this->issue($settings, $keySha, $bindingSha, $command, $binding);
        }
    }

    /**
     * Counts this issuer's pending intents while holding the lock that serializes issuers
     * against the backlog bound. MariaDB takes next-key locks through a locking count.
     * PostgreSQL rejects FOR UPDATE with an aggregate and has no gap locks, so it takes a
     * transaction-scoped advisory lock per issuer before a plain count (ADR 0046).
     *
     * The same lock is held from before the row id (the `source_revision`) is drawn until the
     * outermost transaction ends, so issuers commit in `source_revision` order (Game U-CC9).
     */
    private function lockedPendingCount(): int
    {
        $query = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('state', CharacterCommandIntentRow::PENDING);

        if (DB::connection()->getDriverName() === 'pgsql') {
            DB::select('select pg_advisory_xact_lock(hashtextextended(?, 0))', [
                CharacterCommandIntentRow::TABLE.':pending:'.CharacterCommandWire::ISSUER_AUTHORITY,
            ]);

            return $query->count();
        }

        return $query->lockForUpdate()->count();
    }

    private function existingIntent(string $keySha, string $bindingSha, bool $locking): ?CharacterCommandStatus
    {
        $query = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('request_key_sha256', $keySha);
        $row = ($locking ? $query->lockForUpdate() : $query)->first();
        if (! $row instanceof stdClass) {
            return null;
        }

        $intent = CharacterCommandIntentRow::fromRow($row);
        if (! hash_equals($intent->bindingSha256, $bindingSha)) {
            throw new CharacterCommandConflict('Idempotency key is already bound to a different command intent.');
        }

        return $this->statusOf($intent);
    }

    /** @param array<string, string> $binding */
    private function issue(CharacterCommandSettings $settings, string $keySha, string $bindingSha, string $command, array $binding): CharacterCommandStatus
    {
        return DB::transaction(function () use ($settings, $keySha, $bindingSha, $command, $binding): CharacterCommandStatus {
            $existing = $this->existingIntent($keySha, $bindingSha, false);
            if ($existing !== null) {
                return $existing;
            }

            // The locking count serializes issuers against the backlog bound (CHARCMD-PENDING-BACKLOG).
            $pending = $this->lockedPendingCount();
            // A same-key request may have committed while this one waited for the lock: answer
            // with its intent before enforcing the bound. The locking read sees the latest
            // committed row on both engines (MariaDB's plain read would reuse the snapshot).
            $existing = $this->existingIntent($keySha, $bindingSha, true);
            if ($existing !== null) {
                return $existing;
            }
            if ($pending >= $settings->maxPendingIntents) {
                throw new CharacterCommandBacklogFull('Character command backlog is full; retry later.');
            }

            $issuedAt = now()->getTimestamp();
            $expiresAt = $issuedAt + $settings->intentTtlSeconds;
            $operationId = CanonicalAccountId::generate();
            $timestamp = now();
            // The row id is the source_revision: unique per (issuer_authority, variant) and positive.
            $id = DB::table(CharacterCommandIntentRow::TABLE)->insertGetId([
                'issuer_authority' => CharacterCommandWire::ISSUER_AUTHORITY,
                'operation_id' => $operationId,
                'command' => $command,
                'request_key_sha256' => $keySha,
                'binding_sha256' => $bindingSha,
                'issued_at_source' => $issuedAt,
                'expires_at_source' => $expiresAt,
                'state' => CharacterCommandIntentRow::PENDING,
                'created_at' => $timestamp,
                'updated_at' => $timestamp,
            ]);
            $this->assertAboveEveryStoredKey($id, $operationId, $command);
            $json = CharacterCommandWire::encodeIntent($command, CanonicalAccountId::generate(), $id, $operationId, $binding, $issuedAt, $expiresAt);
            DB::table(CharacterCommandIntentRow::TABLE)->where('id', $id)->update([
                'intent_json' => $json,
                'intent_sha256' => hash('sha256', $json),
            ]);

            return new CharacterCommandStatus($operationId, $command, CharacterCommandState::Pending, null, $expiresAt, null);
        }, 3);
    }

    /**
     * Game U-CC9: Platform issuance commits no intent behind the Game consumer's cursor. The
     * consumer's cursor is always the key of an intent it has listed, so it is a committed key;
     * an intent whose `source_revision` lies above every stored key of this issuer, committed
     * while the issuer lock still serializes every other issuer, lands ahead of it. The sequence
     * that draws the row id normally guarantees this; a sequence left behind its table (a copy or
     * restore that skipped `setval`, a hand-written row) would not, so the intent is refused
     * before it is listable and the operator is alarmed. The read must see every committed row:
     * a plain read does under PostgreSQL READ COMMITTED; MariaDB needs a locking (current) read.
     */
    private function assertAboveEveryStoredKey(int $id, string $operationId, string $command): void
    {
        $query = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('id', '>', $id);
        if (DB::connection()->getDriverName() !== 'pgsql') {
            $query->sharedLock();
        }
        if ($query->value('id') !== null) {
            Log::error('character_authority.command_source_revision_behind', [
                'operation_id' => $operationId,
                'command' => $command,
            ]);

            throw new CharacterCommandUnavailable('Character command source_revision would not lie above every stored key.');
        }
    }

    private function statusOf(CharacterCommandIntentRow $intent): CharacterCommandStatus
    {
        if ($intent->currentReceiptId === null) {
            return CharacterCommandStatus::of($intent, null);
        }
        $row = DB::table(CharacterCommandStoredReceipt::TABLE)->where('id', $intent->currentReceiptId)->first();
        if (! $row instanceof stdClass) {
            throw new CharacterCommandUnavailable('Stored Character command receipt is missing.');
        }

        return CharacterCommandStatus::of($intent, CharacterCommandStoredReceipt::fromRow($row, $intent->id));
    }
}
