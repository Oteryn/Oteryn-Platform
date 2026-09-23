<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use JsonException;
use stdClass;

final class CharacterBootstrapIntentIssuer
{
    private const AUTHORITY_ROW_ID = 1;

    /**
     * @param  array{target_world_id:string,profile_revision:string,ruleset_revision:string,content_revision:string,starter_template_revision:string}  $binding
     * @return array<string, int|string|array<string, string>>
     */
    public function issue(int $identityId, string $operationId, array $binding): array
    {
        if ($identityId < 1) {
            throw new CharacterBootstrapIntentUnavailable('Identity lookup input must be positive.');
        }
        CharacterBootstrapIntentContract::assertUuid($operationId, 'operation_id');
        CharacterBootstrapIntentContract::assertUuid($binding['target_world_id'], 'target_world_id');
        foreach (['profile_revision', 'ruleset_revision', 'content_revision', 'starter_template_revision'] as $field) {
            CharacterBootstrapIntentContract::assertRevision($binding[$field], $field);
        }

        return DB::transaction(function () use ($identityId, $operationId, $binding): array {
            $authority = DB::table('character_bootstrap_intent_authority')
                ->where('id', self::AUTHORITY_ROW_ID)
                ->lockForUpdate()
                ->first();
            if (! $authority instanceof stdClass) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent authority state is unavailable.');
            }

            $identity = DB::table('identities')->where('id', $identityId)->lockForUpdate()->first();
            if (! $identity instanceof stdClass || ! is_string($identity->account_id ?? null)) {
                throw new CharacterBootstrapIntentUnavailable('Platform Identity has no canonical AccountId.');
            }
            CharacterBootstrapIntentContract::assertAccountId($identity->account_id);
            if (($identity->disabled_at ?? null) !== null || ($identity->terminated_at ?? null) !== null) {
                throw new CharacterBootstrapIntentUnavailable('Platform Identity is not eligible for bootstrap issuance.');
            }

            $existing = DB::table('character_bootstrap_intents')->where('operation_id', $operationId)->first();
            if ($existing instanceof stdClass) {
                $this->assertSameBinding($existing, $identityId, $identity->account_id, $binding);

                return $this->decodeStored($existing);
            }

            $lastRevision = $this->positiveOrZero($authority->last_source_revision ?? null);
            if ($lastRevision >= PHP_INT_MAX) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source revision is exhausted.');
            }
            $sourceRevision = $lastRevision + 1;
            $ttl = $this->configuredTtl();
            $issuedAt = now()->getTimestamp();
            if ($issuedAt < 0 || $issuedAt > PHP_INT_MAX - $ttl) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source time is invalid.');
            }
            $decisionId = strtolower((string) Str::uuid());
            $payload = [
                'contract_version' => CharacterBootstrapIntentContract::CONTRACT_VERSION,
                'variant' => CharacterBootstrapIntentContract::VARIANT,
                'issuer_authority' => CharacterBootstrapIntentContract::ISSUER_AUTHORITY,
                'issuer_decision_id' => $decisionId,
                'source_revision' => (string) $sourceRevision,
                'operation_id' => $operationId,
                'operation' => CharacterBootstrapIntentContract::OPERATION,
                'account_id' => $identity->account_id,
                'target_world_id' => $binding['target_world_id'],
                'interpretation_context' => [
                    'profile_revision' => $binding['profile_revision'],
                    'ruleset_revision' => $binding['ruleset_revision'],
                    'content_revision' => $binding['content_revision'],
                    'starter_template_revision' => $binding['starter_template_revision'],
                ],
                'issued_at_source' => (string) $issuedAt,
                'expires_at_source' => (string) ($issuedAt + $ttl),
                'audience' => CharacterBootstrapIntentContract::AUDIENCE,
            ];
            $json = json_encode($payload, JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
            if (strlen($json) > CharacterBootstrapIntentContract::MAX_RESPONSE_BYTES) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent response exceeds its bound.');
            }

            DB::table('character_bootstrap_intents')->insert([
                'identity_id' => $identityId,
                'operation_id' => $operationId,
                'issuer_decision_id' => $decisionId,
                'source_revision' => $sourceRevision,
                'account_id' => $identity->account_id,
                'target_world_id' => $binding['target_world_id'],
                'profile_revision' => $binding['profile_revision'],
                'ruleset_revision' => $binding['ruleset_revision'],
                'content_revision' => $binding['content_revision'],
                'starter_template_revision' => $binding['starter_template_revision'],
                'issued_at_source' => $issuedAt,
                'expires_at_source' => $issuedAt + $ttl,
                'intent_json' => $json,
                'intent_sha256' => hash('sha256', $json),
                'created_at' => now(),
            ]);
            DB::table('character_bootstrap_intent_authority')->where('id', self::AUTHORITY_ROW_ID)->update([
                'last_source_revision' => $sourceRevision,
                'last_issuer_decision_id' => $decisionId,
                'updated_at' => now(),
            ]);

            return $payload;
        }, 3);
    }

    /** @param array<string, string> $binding */
    private function assertSameBinding(stdClass $row, int $identityId, string $accountId, array $binding): void
    {
        $storedIdentityId = filter_var($row->identity_id ?? null, FILTER_VALIDATE_INT, ['options' => ['min_range' => 1]]);
        $stored = [];
        foreach (['account_id', 'target_world_id', 'profile_revision', 'ruleset_revision', 'content_revision', 'starter_template_revision'] as $field) {
            $value = $row->{$field} ?? null;
            if (! is_string($value)) {
                throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap-intent binding is invalid.');
            }
            $stored[$field] = $value;
        }

        $same = $storedIdentityId === $identityId
            && hash_equals($stored['account_id'], $accountId)
            && hash_equals($stored['target_world_id'], $binding['target_world_id'])
            && hash_equals($stored['profile_revision'], $binding['profile_revision'])
            && hash_equals($stored['ruleset_revision'], $binding['ruleset_revision'])
            && hash_equals($stored['content_revision'], $binding['content_revision'])
            && hash_equals($stored['starter_template_revision'], $binding['starter_template_revision']);
        if (! $same) {
            throw new CharacterBootstrapIntentConflict('operation_id is already bound to a different immutable intent.');
        }
    }

    /** @return array<string, int|string|array<string, string>> */
    private function decodeStored(stdClass $row): array
    {
        if (! is_string($row->intent_json ?? null)
            || ! is_string($row->intent_sha256 ?? null)
            || ! hash_equals($row->intent_sha256, hash('sha256', $row->intent_json))) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }
        try {
            $payload = json_decode($row->intent_json, true, 3, JSON_THROW_ON_ERROR);
        } catch (JsonException $exception) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.', 0, $exception);
        }
        if (! is_array($payload) || array_is_list($payload)) {
            throw new CharacterBootstrapIntentUnavailable('Stored Character bootstrap intent is invalid.');
        }

        /** @var array<string, int|string|array<string, string>> $payload */
        return $payload;
    }

    private function positiveOrZero(mixed $value): int
    {
        if (! is_int($value) && (! is_string($value) || preg_match('/\A(0|[1-9][0-9]*)\z/D', $value) !== 1)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source state is invalid.');
        }
        $parsed = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 0]]);
        if (! is_int($parsed)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent source state is invalid.');
        }

        return $parsed;
    }

    private function configuredTtl(): int
    {
        $value = config('game-auth.character_bootstrap_intent.ttl_seconds');
        if (! is_int($value) && (! is_string($value) || preg_match('/\A[1-9][0-9]{0,2}\z/D', $value) !== 1)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent TTL is not configured.');
        }
        $ttl = filter_var($value, FILTER_VALIDATE_INT, ['options' => ['min_range' => 1, 'max_range' => 300]]);
        if (! is_int($ttl)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap-intent TTL is outside the technical bound.');
        }

        return $ttl;
    }
}
