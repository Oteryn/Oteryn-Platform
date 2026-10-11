<?php

namespace App\CharacterAuthority;

use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Log;
use stdClass;

/**
 * Platform side of the three Game-initiated exchanges (Game contract §4): list PENDING intents,
 * read one intent's exact bytes, and store Game's receipt. Logs carry only the operation, command
 * and result code, never an AccountId or name (§11).
 */
final class CharacterCommandExchange
{
    private const INT64_MAX = '9223372036854775807';

    public function pending(CharacterCommandSettings $settings, string $raw): string
    {
        $request = CharacterCommandWire::decodeListRequest($raw);
        $this->assertSourceAuthority($settings, $request['source_authority']);

        $after = $request['after_source_revision'];
        if (strlen($after) > strlen(self::INT64_MAX) || (strlen($after) === strlen(self::INT64_MAX) && strcmp($after, self::INT64_MAX) > 0)) {
            // No stored source_revision can lie above a key beyond the signed row-id range.
            return CharacterCommandWire::encodePendingList([]);
        }
        $afterId = (int) $after;
        $afterOperationId = $request['after_operation_id'];

        $rows = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
            ->where('state', CharacterCommandIntentRow::PENDING)
            ->whereNotNull('intent_json')
            ->where(fn ($query) => $query
                ->where('id', '>', $afterId)
                ->orWhere(fn ($same) => $same->where('id', $afterId)->where('operation_id', '>', $afterOperationId)))
            ->orderBy('id')
            ->orderBy('operation_id')
            ->limit($request['max_entries'])
            ->get();

        $entries = [];
        foreach ($rows as $row) {
            $intent = CharacterCommandIntentRow::fromRow($row);
            $entries[] = [
                'command' => $intent->command,
                'operation_id' => $intent->operationId,
                'source_revision' => (string) $intent->id,
            ];
        }

        return CharacterCommandWire::encodePendingList($entries);
    }

    public function read(CharacterCommandSettings $settings, string $raw): string
    {
        $request = CharacterCommandWire::decodeReadRequest($raw);
        $this->assertSourceAuthority($settings, $request['source_authority']);

        $row = DB::table(CharacterCommandIntentRow::TABLE)
            ->where('issuer_authority', $request['issuer_authority'])
            ->where('operation_id', $request['operation_id'])
            ->whereNotNull('intent_json')
            ->first();
        if (! $row instanceof stdClass) {
            throw new CharacterCommandRefused(404, 'No Character command intent for that operation.');
        }
        $intent = CharacterCommandIntentRow::fromRow($row);
        if ($intent->command !== $request['command']) {
            $this->alarm('character_authority.command_read_command_mismatch', $intent->operationId, $request['command'], null);

            throw new CharacterCommandRefused(409, 'Character command intent holds another command.');
        }

        return $intent->intentJson;
    }

    public function receipt(CharacterCommandSettings $settings, string $raw): void
    {
        $receipt = CharacterCommandWire::decodeReceipt($raw);
        $this->assertSourceAuthority($settings, $receipt->sourceAuthority);

        DB::transaction(function () use ($receipt, $raw): void {
            $row = DB::table(CharacterCommandIntentRow::TABLE)
                ->where('issuer_authority', CharacterCommandWire::ISSUER_AUTHORITY)
                ->where('operation_id', $receipt->operationId)
                ->whereNotNull('intent_json')
                ->lockForUpdate()
                ->first();
            if (! $row instanceof stdClass) {
                $this->alarm('character_authority.command_receipt_without_intent', $receipt->operationId, $receipt->command, $receipt->resultCode);

                throw new CharacterCommandRefused(404, 'No Character command intent for that operation.');
            }
            $intent = CharacterCommandIntentRow::fromRow($row);
            if ($intent->command !== $receipt->command) {
                $this->conflict($intent, $receipt, 'character_authority.command_receipt_command_mismatch');
            }

            if ($intent->currentReceiptId !== null) {
                $stored = DB::table(CharacterCommandStoredReceipt::TABLE)->where('id', $intent->currentReceiptId)->first();
                if (! $stored instanceof stdClass) {
                    throw new CharacterCommandUnavailable('Stored Character command receipt is missing.');
                }
                if (hash_equals(CharacterCommandStoredReceipt::fromRow($stored, $intent->id)->receiptJson, $raw)) {
                    return; // Exact republication: accepted, nothing written.
                }
                $this->conflict($intent, $receipt, 'character_authority.command_receipt_conflict');
            }

            if ($receipt->outcome === CharacterCommandWire::COMMITTED && ! $this->bindingMatches($intent, $receipt)) {
                $this->conflict($intent, $receipt, 'character_authority.command_receipt_binding_mismatch');
            }

            $now = now();
            $receiptId = DB::table(CharacterCommandStoredReceipt::TABLE)->insertGetId([
                'intent_id' => $intent->id,
                'issuer_authority' => CharacterCommandWire::ISSUER_AUTHORITY,
                'operation_id' => $intent->operationId,
                'command' => $intent->command,
                'outcome' => $receipt->outcome,
                'result_code' => $receipt->resultCode,
                // Fixed width so a restore can select `recovery_generation >= H` as text (§5.4).
                'recovery_generation' => str_pad($receipt->recoveryGeneration, 20, '0', STR_PAD_LEFT),
                'receipt_json' => $raw,
                'receipt_sha256' => hash('sha256', $raw),
                'superseded_at' => null,
                'created_at' => $now,
            ]);
            DB::table(CharacterCommandIntentRow::TABLE)->where('id', $intent->id)->update([
                'state' => CharacterCommandIntentRow::TERMINAL,
                'current_receipt_id' => $receiptId,
                'updated_at' => $now,
            ]);

            Log::info('character_authority.command_receipt_stored', [
                'operation_id' => $intent->operationId,
                'command' => $intent->command,
                'result_code' => $receipt->resultCode,
            ]);
        }, 3);
    }

    private function bindingMatches(CharacterCommandIntentRow $intent, CharacterCommandReceipt $receipt): bool
    {
        $stored = $intent->intent();
        $expected = $intent->command === CharacterCommandWire::CREATE
            ? [
                'account_id' => $stored['account_id'] ?? null,
                'world_id' => $stored['target_world_id'] ?? null,
                'name' => $stored['requested_name'] ?? null,
            ]
            : [
                'character_id' => $stored['character_id'] ?? null,
                'from_account_id' => $stored['expected_from_account_id'] ?? null,
                'to_account_id' => $stored['to_account_id'] ?? null,
            ];

        foreach ($expected as $member => $value) {
            if (! is_string($value) || ($receipt->fields[$member] ?? null) !== $value) {
                return false;
            }
        }

        return true;
    }

    private function assertSourceAuthority(CharacterCommandSettings $settings, string $sourceAuthority): void
    {
        if (! hash_equals($settings->sourceAuthority, $sourceAuthority)) {
            throw new CharacterCommandRefused(401, 'Character command source authority is not configured.');
        }
    }

    /** Different bytes or a contradicting receipt: the intent stays as it is and both sides alarm (§4.3). */
    private function conflict(CharacterCommandIntentRow $intent, CharacterCommandReceipt $receipt, string $event): never
    {
        $this->alarm($event, $intent->operationId, $receipt->command, $receipt->resultCode);

        throw new CharacterCommandRefused(409, 'Character command receipt conflicts with the stored state.');
    }

    private function alarm(string $event, string $operationId, string $command, ?string $resultCode): void
    {
        Log::error($event, ['operation_id' => $operationId, 'command' => $command, 'result_code' => $resultCode]);
    }
}
