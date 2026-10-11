<?php

namespace App\CharacterAuthority;

use App\GameAuth\CharacterBootstrapIntent\CharacterBootstrapIntentContract;
use App\GameAuth\NativeAccountCharacters\NativeAccountCharactersWire;
use InvalidArgumentException;

/**
 * Wire of Game contract `oteryn-game-character-authority-commands-v1` (Game PR #1936, §§3, 4, 7, 12).
 * Platform stores intents; the Game command consumer pulls them and publishes receipts. Platform never
 * calls Game.
 */
final class CharacterCommandWire
{
    public const CONTRACT_VERSION = 1;

    public const ISSUER_AUTHORITY = 'OTERYN_PLATFORM_CHARACTER_AUTHORITY';

    public const AUDIENCE = 'OTERYN_GAME_CHARACTER_AUTHORITY';

    public const CREATE = 'CreateCharacter';

    public const TRANSFER = 'TransferCharacterOwnership';

    public const VARIANTS = [
        self::CREATE => 'PLATFORM_USER_CREATE',
        self::TRANSFER => 'PLATFORM_BAZAAR_TRANSFER',
    ];

    public const LIST_PATH = '/internal/v1/game-auth/character-commands/pending';

    public const READ_PATH = '/internal/v1/game-auth/character-commands/read';

    public const RECEIPT_PATH = '/internal/v1/game-auth/character-commands/receipt';

    public const MAX_LIST_REQUEST_BYTES = 512;

    public const MAX_READ_REQUEST_BYTES = 512;

    public const MAX_RECEIPT_REQUEST_BYTES = 1024;

    public const MAX_INTENT_BYTES = 4096;

    public const MAX_LIST_RESPONSE_BYTES = 8192;

    public const MAX_LIST_ENTRIES = 32;

    /** `CHARCMD-PENDING-BACKLOG`: unreceipted PENDING intents Platform may hold. */
    public const MAX_PENDING_BACKLOG = 256;

    public const MAX_TTL_SECONDS = 300;

    public const COMMITTED = 'COMMITTED';

    public const REJECTED = 'REJECTED';

    public const OK = 'CHAR_CMD_OK';

    public const UNSUPPORTED = 'CHAR_CMD_UNSUPPORTED';

    public const ACCEPTED_RESPONSE = '{"contract_version":1,"result":"accepted"}';

    private const NIL_UUID = '00000000-0000-0000-0000-000000000000';

    private const UUID7 = '/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/D';

    private const COMMAND = '/^[A-Za-z0-9_]{1,64}$/D';

    private const ISSUER = '/^[A-Z0-9_]{1,128}$/D';

    private const SOURCE_AUTHORITY = '/^[A-Za-z0-9._:\/-]{1,128}$/D';

    /** §7: every REJECTED code with the commands it may answer. */
    private const REJECTION_CODES = [
        'CHAR_CMD_NAME_INVALID' => [self::CREATE],
        'CHAR_CMD_NAME_UNAVAILABLE' => [self::CREATE],
        'CHAR_CMD_WORLD_NOT_ACCEPTED' => [self::CREATE],
        'CHAR_CMD_ACCOUNT_CHARACTER_LIMIT' => [self::CREATE, self::TRANSFER],
        'CHAR_CMD_ACCOUNT_NOT_ELIGIBLE' => [self::CREATE, self::TRANSFER],
        'CHAR_CMD_CHARACTER_NOT_FOUND' => [self::TRANSFER],
        'CHAR_CMD_OWNER_MISMATCH' => [self::TRANSFER],
        'CHAR_CMD_CHARACTER_IN_SESSION' => [self::TRANSFER],
        'CHAR_CMD_CHARACTER_NOT_TRANSFERABLE' => [self::TRANSFER],
        'CHAR_CMD_HOUSING_NOT_PREPARED' => [self::TRANSFER],
        'CHAR_CMD_SAME_ACCOUNT' => [self::TRANSFER],
        'CHAR_CMD_INTENT_EXPIRED' => [self::CREATE, self::TRANSFER],
        'CHAR_CMD_INTENT_INVALID' => [self::CREATE, self::TRANSFER],
        'CHAR_CMD_OPERATION_CONFLICT' => [self::CREATE, self::TRANSFER],
        self::UNSUPPORTED => [self::CREATE, self::TRANSFER],
    ];

    private const RECEIPT_HEAD = ['contract_version', 'operation', 'source_authority', 'command', 'operation_id', 'outcome', 'result_code'];

    private const RECEIPT_TAIL = ['recovery_generation', 'decided_at'];

    /** Committed members beyond the common head and tail, per command (§4.3). */
    private const COMMITTED_MEMBERS = [
        self::CREATE => [
            'character_id' => 'uuid', 'account_id' => 'uuid', 'world_id' => 'uuid', 'name' => 'name',
            'character_revision' => 'uint64', 'projection_epoch' => 'uint64', 'projection_revision' => 'uint64',
        ],
        self::TRANSFER => [
            'character_id' => 'uuid', 'from_account_id' => 'uuid', 'to_account_id' => 'uuid',
            'character_revision' => 'uint64', 'projection_epoch' => 'uint64',
            'from_projection_revision' => 'uint64', 'to_projection_revision' => 'uint64',
        ],
    ];

    public static function assertUuid7(string $value, string $label): void
    {
        if (preg_match(self::UUID7, $value) !== 1) {
            throw new InvalidArgumentException("{$label} must be a canonical lower-case UUIDv7.");
        }
    }

    /** Advisory CHAR-NAME-1 syntax check; Game validates and reserves the name. */
    public static function assertRequestedName(string $value): void
    {
        CharacterBootstrapIntentContract::assertRequestedName($value);
    }

    public static function isSourceAuthority(mixed $value): bool
    {
        return is_string($value) && preg_match(self::SOURCE_AUTHORITY, $value) === 1;
    }

    /**
     * Canonical intent bytes (§4.2). Member order follows the contract examples.
     *
     * @param  array<string, string>  $binding  command-specific members in wire order
     */
    public static function encodeIntent(string $command, string $decisionId, int $sourceRevision, string $operationId, array $binding, int $issuedAt, int $expiresAt): string
    {
        $variant = self::VARIANTS[$command] ?? throw new InvalidArgumentException('Unsupported Character Authority command.');
        if ($sourceRevision < 1 || $issuedAt < 1 || $expiresAt <= $issuedAt || $expiresAt - $issuedAt > self::MAX_TTL_SECONDS) {
            throw new InvalidArgumentException('Character command intent ordering or lifetime is invalid.');
        }

        $intent = [
            'contract_version' => self::CONTRACT_VERSION,
            'command' => $command,
            'variant' => $variant,
            'issuer_authority' => self::ISSUER_AUTHORITY,
            'issuer_decision_id' => $decisionId,
            'source_revision' => (string) $sourceRevision,
            'operation_id' => $operationId,
            ...$binding,
            'issued_at_source' => (string) $issuedAt,
            'expires_at_source' => (string) $expiresAt,
            'audience' => self::AUDIENCE,
        ];
        $json = json_encode($intent, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if (strlen($json) > self::MAX_INTENT_BYTES || str_contains($json, '\\')) {
            throw new InvalidArgumentException('Character command intent is not representable on the wire.');
        }

        return $json;
    }

    /** @param list<array{command:string,operation_id:string,source_revision:string}> $entries */
    public static function encodePendingList(array $entries): string
    {
        $json = json_encode(['contract_version' => self::CONTRACT_VERSION, 'pending' => $entries], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES);
        if (count($entries) > self::MAX_LIST_ENTRIES || strlen($json) > self::MAX_LIST_RESPONSE_BYTES) {
            throw new InvalidArgumentException('Character command pending list exceeds its bound.');
        }

        return $json;
    }

    /** @return array{source_authority:string,after_source_revision:string,after_operation_id:string,max_entries:int} */
    public static function decodeListRequest(string $raw): array
    {
        $request = self::decode($raw, self::MAX_LIST_REQUEST_BYTES);
        NativeAccountCharactersWire::exactKeys($request, ['contract_version', 'operation', 'source_authority', 'after_source_revision', 'after_operation_id', 'max_entries']);
        self::assertHead($request, 'ListPendingCharacterCommandsV1');

        $after = $request['after_source_revision'];
        if ($after !== '0') {
            $after = NativeAccountCharactersWire::uint64($request, 'after_source_revision');
        }
        $afterOperationId = $request['after_operation_id'];
        if ($afterOperationId !== self::NIL_UUID) {
            $afterOperationId = NativeAccountCharactersWire::uuid7($request, 'after_operation_id');
        }
        $max = $request['max_entries'];
        if (! is_int($max) || $max < 1 || $max > self::MAX_LIST_ENTRIES) {
            throw new InvalidArgumentException('max_entries is invalid.');
        }

        return [
            'source_authority' => NativeAccountCharactersWire::authority($request),
            'after_source_revision' => $after,
            'after_operation_id' => $afterOperationId,
            'max_entries' => $max,
        ];
    }

    /** @return array{source_authority:string,issuer_authority:string,command:string,operation_id:string} */
    public static function decodeReadRequest(string $raw): array
    {
        $request = self::decode($raw, self::MAX_READ_REQUEST_BYTES);
        NativeAccountCharactersWire::exactKeys($request, ['contract_version', 'operation', 'source_authority', 'issuer_authority', 'command', 'operation_id']);
        self::assertHead($request, 'ReadCharacterCommandIntentV1');

        return [
            'source_authority' => NativeAccountCharactersWire::authority($request),
            'issuer_authority' => NativeAccountCharactersWire::text($request, 'issuer_authority', self::ISSUER),
            'command' => NativeAccountCharactersWire::text($request, 'command', self::COMMAND),
            'operation_id' => NativeAccountCharactersWire::uuid7($request, 'operation_id'),
        ];
    }

    public static function decodeReceipt(string $raw): CharacterCommandReceipt
    {
        $receipt = self::decode($raw, self::MAX_RECEIPT_REQUEST_BYTES);
        self::assertHead($receipt, 'PublishCharacterCommandReceiptV1');
        $command = NativeAccountCharactersWire::text($receipt, 'command', self::COMMAND);
        $outcome = $receipt['outcome'] ?? null;
        $resultCode = $receipt['result_code'] ?? null;
        if (! is_string($outcome) || ! is_string($resultCode)) {
            throw new InvalidArgumentException('Character command receipt outcome is invalid.');
        }

        $members = [];
        if ($outcome === self::COMMITTED && $resultCode === self::OK && isset(self::COMMITTED_MEMBERS[$command])) {
            $members = self::COMMITTED_MEMBERS[$command];
        } elseif ($outcome !== self::REJECTED
            || ! isset(self::REJECTION_CODES[$resultCode])
            // A command Platform does not know can only be answered CHAR_CMD_UNSUPPORTED (§4.3, §7).
            || ($resultCode !== self::UNSUPPORTED && ! in_array($command, self::REJECTION_CODES[$resultCode], true))) {
            throw new InvalidArgumentException('Character command receipt outcome is invalid.');
        }
        NativeAccountCharactersWire::exactKeys($receipt, [...self::RECEIPT_HEAD, ...array_keys($members), ...self::RECEIPT_TAIL]);

        $fields = [];
        foreach ($members as $member => $type) {
            $fields[$member] = match ($type) {
                'uuid' => NativeAccountCharactersWire::uuid7($receipt, $member),
                'name' => NativeAccountCharactersWire::characterName($receipt[$member]),
                default => NativeAccountCharactersWire::uint64($receipt, $member),
            };
        }

        return new CharacterCommandReceipt(
            sourceAuthority: NativeAccountCharactersWire::authority($receipt),
            command: $command,
            operationId: NativeAccountCharactersWire::uuid7($receipt, 'operation_id'),
            outcome: $outcome,
            resultCode: $resultCode,
            recoveryGeneration: NativeAccountCharactersWire::uint64($receipt, 'recovery_generation'),
            decidedAt: NativeAccountCharactersWire::uint64($receipt, 'decided_at'),
            fields: $fields,
        );
    }

    /** @return array<string, mixed> */
    private static function decode(string $raw, int $maxBytes): array
    {
        return NativeAccountCharactersWire::decodeExact($raw, $maxBytes);
    }

    /** @param array<string, mixed> $object */
    private static function assertHead(array $object, string $operation): void
    {
        if (($object['contract_version'] ?? null) !== self::CONTRACT_VERSION || ($object['operation'] ?? null) !== $operation) {
            throw new InvalidArgumentException('Character command contract version or operation is invalid.');
        }
    }
}
