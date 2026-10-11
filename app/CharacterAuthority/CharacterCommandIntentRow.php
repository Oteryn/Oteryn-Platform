<?php

namespace App\CharacterAuthority;

use stdClass;

/** A validated `character_command_intents` row; corruption fails closed as unavailable. */
final readonly class CharacterCommandIntentRow
{
    public const TABLE = 'character_command_intents';

    public const PENDING = 'PENDING';

    public const TERMINAL = 'TERMINAL';

    private function __construct(
        public int $id,
        public string $operationId,
        public string $command,
        public string $state,
        public int $expiresAtSource,
        public string $intentJson,
        public string $bindingSha256,
        public ?int $currentReceiptId,
    ) {}

    public static function fromRow(stdClass $row): self
    {
        $id = self::positiveInt($row->id ?? null);
        $expiresAt = self::positiveInt($row->expires_at_source ?? null);
        $receiptId = ($row->current_receipt_id ?? null) === null ? null : self::positiveInt($row->current_receipt_id);
        $operationId = $row->operation_id ?? null;
        $command = $row->command ?? null;
        $state = $row->state ?? null;
        $json = $row->intent_json ?? null;
        $sha = $row->intent_sha256 ?? null;
        $binding = $row->binding_sha256 ?? null;

        if ($id === null || $expiresAt === null
            || ! is_string($operationId) || ! is_string($command) || ! is_string($state)
            || ! is_string($json) || ! is_string($sha) || ! is_string($binding)
            || ! isset(CharacterCommandWire::VARIANTS[$command])
            || strlen($json) > CharacterCommandWire::MAX_INTENT_BYTES
            || preg_match('/^[0-9a-f]{64}$/D', $binding) !== 1
            || ! hash_equals($sha, hash('sha256', $json))
            || ! (($state === self::PENDING && $receiptId === null) || ($state === self::TERMINAL && $receiptId !== null))) {
            throw new CharacterCommandUnavailable('Stored Character command intent is invalid.');
        }

        return new self($id, $operationId, $command, $state, $expiresAt, $json, $binding, $receiptId);
    }

    /** @return array<string, int|string> */
    public function intent(): array
    {
        $intent = json_decode($this->intentJson, true, 2);
        if (! is_array($intent) || ($intent['operation_id'] ?? null) !== $this->operationId || ($intent['command'] ?? null) !== $this->command) {
            throw new CharacterCommandUnavailable('Stored Character command intent is invalid.');
        }

        /** @var array<string, int|string> $intent */
        return $intent;
    }

    private static function positiveInt(mixed $value): ?int
    {
        $parsed = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);

        return is_int($parsed) ? $parsed : null;
    }
}
