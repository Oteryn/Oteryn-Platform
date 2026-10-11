<?php

namespace App\CharacterAuthority;

use stdClass;

/** A validated `character_command_receipts` row: the exact bytes Game published. */
final readonly class CharacterCommandStoredReceipt
{
    public const TABLE = 'character_command_receipts';

    private function __construct(
        public string $outcome,
        public string $resultCode,
        public string $receiptJson,
    ) {}

    public static function fromRow(stdClass $row, int $intentId): self
    {
        $outcome = $row->outcome ?? null;
        $resultCode = $row->result_code ?? null;
        $json = $row->receipt_json ?? null;
        $sha = $row->receipt_sha256 ?? null;
        if (filter_var($row->intent_id ?? null, FILTER_VALIDATE_INT) !== $intentId
            || ! in_array($outcome, [CharacterCommandWire::COMMITTED, CharacterCommandWire::REJECTED], true)
            || ! is_string($resultCode) || ! is_string($json) || ! is_string($sha)
            || strlen($json) > CharacterCommandWire::MAX_RECEIPT_REQUEST_BYTES
            || ! hash_equals($sha, hash('sha256', $json))) {
            throw new CharacterCommandUnavailable('Stored Character command receipt is invalid.');
        }

        return new self($outcome, $resultCode, $json);
    }

    /** @return array<string, int|string> */
    public function receipt(): array
    {
        $receipt = json_decode($this->receiptJson, true, 2);
        if (! is_array($receipt)) {
            throw new CharacterCommandUnavailable('Stored Character command receipt is invalid.');
        }

        /** @var array<string, int|string> $receipt */
        return $receipt;
    }
}
