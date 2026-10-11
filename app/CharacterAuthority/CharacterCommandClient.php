<?php

namespace App\CharacterAuthority;

use App\Identity\Support\CanonicalAccountId;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Support\Facades\DB;
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

    /** @param array<string, string> $binding */
    private function issue(CharacterCommandSettings $settings, string $keySha, string $bindingSha, string $command, array $binding): CharacterCommandStatus
    {
        return DB::transaction(function () use ($settings, $keySha, $bindingSha, $command, $binding): CharacterCommandStatus {
            $existing = DB::table(CharacterCommandIntentRow::TABLE)
                ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
                ->where('request_key_sha256', $keySha)
                ->first();
            if ($existing instanceof stdClass) {
                $intent = CharacterCommandIntentRow::fromRow($existing);
                if (! hash_equals($intent->bindingSha256, $bindingSha)) {
                    throw new CharacterCommandConflict('Idempotency key is already bound to a different command intent.');
                }

                return $this->statusOf($intent);
            }

            // The locking count serializes issuers against the backlog bound (CHARCMD-PENDING-BACKLOG).
            $pending = DB::table(CharacterCommandIntentRow::TABLE)
                ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
                ->where('state', CharacterCommandIntentRow::PENDING)
                ->lockForUpdate()
                ->count();
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
            $json = CharacterCommandWire::encodeIntent($command, CanonicalAccountId::generate(), $id, $operationId, $binding, $issuedAt, $expiresAt);
            DB::table(CharacterCommandIntentRow::TABLE)->where('id', $id)->update([
                'intent_json' => $json,
                'intent_sha256' => hash('sha256', $json),
            ]);

            return new CharacterCommandStatus($operationId, $command, CharacterCommandState::Pending, null, $expiresAt, null);
        }, 3);
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
