<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use App\Identity\Support\CanonicalAccountId;
use InvalidArgumentException;
use JsonException;
use stdClass;

final class CharacterBootstrapIntentContract
{
    public const CONTRACT_VERSION = 1;

    public const VARIANT = 'OPERATOR_CONTROL_PLANE_BOOTSTRAP';

    public const ISSUER_AUTHORITY = 'OTERYN_PLATFORM_CHARACTER_AUTHORITY';

    public const OPERATION = 'INITIAL_CHARACTER_BOOTSTRAP';

    public const AUDIENCE = 'OTERYN_GAME_CHARACTER_AUTHORITY';

    public const MAX_REQUEST_BYTES = 256;

    public const MAX_RESPONSE_BYTES = 4096;

    public const MAX_REVISION_BYTES = 128;

    private const UUID_PATTERN = '/\A[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/D';

    private const REVISION_PATTERN = '/\A[A-Za-z0-9][A-Za-z0-9._:-]{0,127}\z/D';

    public static function assertUuid(string $value, string $label): void
    {
        if (preg_match(self::UUID_PATTERN, $value) !== 1) {
            throw new InvalidArgumentException("{$label} must be a canonical lower-case UUID.");
        }
    }

    public static function assertAccountId(string $value): void
    {
        if (! CanonicalAccountId::isValid($value)) {
            throw new InvalidArgumentException('Persisted AccountId is not a canonical Platform AccountId.');
        }
    }

    public static function assertRevision(string $value, string $label): void
    {
        if (strlen($value) > self::MAX_REVISION_BYTES || preg_match(self::REVISION_PATTERN, $value) !== 1) {
            throw new InvalidArgumentException("{$label} is invalid.");
        }
    }

    /** @return array<string, int|string|array<string, string>> */
    public static function decodeStored(stdClass $row): array
    {
        $json = $row->intent_json ?? null;
        $sha = $row->intent_sha256 ?? null;
        if (! is_string($json)
            || strlen($json) > self::MAX_RESPONSE_BYTES
            || ! is_string($sha)
            || preg_match('/\\A[0-9a-f]{64}\\z/D', $sha) !== 1
            || ! hash_equals($sha, hash('sha256', $json))) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        try {
            $payload = json_decode($json, true, 3, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.', 0, $exception);
        }
        if (! is_array($payload) || array_is_list($payload)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        $expectedKeys = [
            'contract_version', 'variant', 'issuer_authority', 'issuer_decision_id', 'source_revision',
            'operation_id', 'operation', 'account_id', 'target_world_id', 'interpretation_context',
            'issued_at_source', 'expires_at_source', 'audience',
        ];
        $keys = array_keys($payload);
        sort($keys);
        sort($expectedKeys);
        if ($keys !== $expectedKeys
            || ($payload['contract_version'] ?? null) !== self::CONTRACT_VERSION
            || ($payload['variant'] ?? null) !== self::VARIANT
            || ($payload['issuer_authority'] ?? null) !== self::ISSUER_AUTHORITY
            || ($payload['operation'] ?? null) !== self::OPERATION
            || ($payload['audience'] ?? null) !== self::AUDIENCE
            || ! is_string($payload['issuer_decision_id'] ?? null)
            || ! is_string($payload['operation_id'] ?? null)
            || ! is_string($payload['account_id'] ?? null)
            || ! is_string($payload['target_world_id'] ?? null)
            || ! is_string($payload['source_revision'] ?? null)
            || ! is_string($payload['issued_at_source'] ?? null)
            || ! is_string($payload['expires_at_source'] ?? null)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        $context = $payload['interpretation_context'] ?? null;
        if (! is_array($context) || array_is_list($context)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }
        $contextKeys = array_keys($context);
        $expectedContextKeys = ['profile_revision', 'ruleset_revision', 'content_revision', 'starter_template_revision'];
        sort($contextKeys);
        sort($expectedContextKeys);
        if ($contextKeys !== $expectedContextKeys) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        try {
            self::assertUuid($payload['issuer_decision_id'], 'issuer_decision_id');
            self::assertUuid($payload['operation_id'], 'operation_id');
            self::assertUuid($payload['target_world_id'], 'target_world_id');
            self::assertAccountId($payload['account_id']);
            foreach ($expectedContextKeys as $field) {
                if (! is_string($context[$field] ?? null)) {
                    throw new InvalidArgumentException("{$field} is invalid.");
                }
                self::assertRevision($context[$field], $field);
            }
        } catch (InvalidArgumentException $exception) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.', 0, $exception);
        }

        $sourceRevision = self::storedDecimal($payload['source_revision'], 1);
        $issuedAt = self::storedDecimal($payload['issued_at_source'], 0);
        $expiresAt = self::storedDecimal($payload['expires_at_source'], 0);
        if ((int) $expiresAt <= (int) $issuedAt || (int) $expiresAt - (int) $issuedAt > 300) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        $stringBindings = [
            'operation_id' => $payload['operation_id'],
            'issuer_decision_id' => $payload['issuer_decision_id'],
            'account_id' => $payload['account_id'],
            'target_world_id' => $payload['target_world_id'],
            'profile_revision' => $context['profile_revision'],
            'ruleset_revision' => $context['ruleset_revision'],
            'content_revision' => $context['content_revision'],
            'starter_template_revision' => $context['starter_template_revision'],
        ];
        foreach ($stringBindings as $field => $expected) {
            $stored = $row->{$field} ?? null;
            if (! is_string($stored) || ! is_string($expected) || ! hash_equals($stored, $expected)) {
                throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent binding is invalid.');
            }
        }
        if (! hash_equals(self::storedDecimal($row->source_revision ?? null, 1), $sourceRevision)
            || ! hash_equals(self::storedDecimal($row->issued_at_source ?? null, 0), $issuedAt)
            || ! hash_equals(self::storedDecimal($row->expires_at_source ?? null, 0), $expiresAt)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent binding is invalid.');
        }

        /** @var array<string, int|string|array<string, string>> $payload */
        return $payload;
    }

    /** @param array<string, int|string|array<string, string>> $payload */
    public static function assertAuthorityState(stdClass $authority, array $payload): void
    {
        $lastRevision = self::storedDecimal($authority->last_source_revision ?? null, 0);
        $sourceRevision = self::storedDecimal($payload['source_revision'] ?? null, 1);
        if ((int) $sourceRevision > (int) $lastRevision) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source ordering is invalid.');
        }
        $lastDecisionId = $authority->last_issuer_decision_id ?? null;
        $decisionId = $payload['issuer_decision_id'] ?? null;
        if (hash_equals($sourceRevision, $lastRevision)
            && (! is_string($lastDecisionId)
                || ! is_string($decisionId)
                || ! hash_equals($decisionId, $lastDecisionId))) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent decision identity conflicts at equal revision.');
        }
    }

    private static function storedDecimal(mixed $value, int $minimum): string
    {
        if (is_int($value)) {
            if ($value < $minimum) {
                throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent numeric state is invalid.');
            }

            return (string) $value;
        }
        if (! is_string($value) || preg_match('/\\A(0|[1-9][0-9]*)\\z/D', $value) !== 1) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent numeric state is invalid.');
        }
        $parsed = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => $minimum]]);
        if (! is_int($parsed)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent numeric state is invalid.');
        }

        return $value;
    }
}
