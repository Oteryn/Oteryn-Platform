<?php

namespace App\GameAuth\CharacterBootstrapIntent;

use App\Identity\Models\Identity;
use Closure;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use stdClass;

final class CharacterBootstrapIntentService
{
    public function __construct(private readonly CharacterBootstrapIntentHighWater $highWater) {}

    /**
     * @param  array{profile_revision: string, ruleset_revision: string, content_revision: string, starter_template_revision: string}  $context
     * @return array<string, int|string|array<string, string>>
     */
    public function issue(int $identityId, string $operationId, string $targetWorldId, array $context): array
    {
        CharacterBootstrapIntentContract::assertUuid($operationId, 'operation_id');
        CharacterBootstrapIntentContract::assertBinding($targetWorldId, 'target_world_id', 64);
        foreach ($context as $field => $revision) {
            CharacterBootstrapIntentContract::assertPositiveRevision($revision, $field);
        }
        $ttl = config('game-auth.character_bootstrap_intent.ttl_seconds');
        $issuer = config('game-auth.character_bootstrap_intent.issuer_authority');
        if (! is_int($ttl) || $ttl < 1 || $ttl > 3600 || ! is_string($issuer)) {
            throw new CharacterBootstrapIntentUnavailable('Character bootstrap authority configuration is unavailable.');
        }
        CharacterBootstrapIntentContract::assertBinding($issuer, 'issuer_authority');

        return $this->highWater->withLock(function (int $floor, Closure $advance) use ($identityId, $operationId, $targetWorldId, $context, $ttl, $issuer): array {
            $databaseHighWater = $this->authorityRevision();
            if ($floor !== $databaseHighWater) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap source ordering is ambiguous.');
            }

            $payload = DB::transaction(function () use ($identityId, $operationId, $targetWorldId, $context, $ttl, $issuer, $databaseHighWater): array {
                $state = DB::table('character_bootstrap_intent_authority_states')->where('id', 1)->lockForUpdate()->first();
                if (! $state instanceof stdClass || $this->positiveDecimalInt($state->source_revision ?? null, 'source_revision', true) !== $databaseHighWater) {
                    throw new CharacterBootstrapIntentUnavailable('Character bootstrap authority state changed unexpectedly.');
                }
                $existing = DB::table('character_bootstrap_intents')->where('operation_id', $operationId)->first();
                if ($existing instanceof stdClass) {
                    return $this->reconcile($existing, $identityId, $targetWorldId, $context);
                }
                $identity = Identity::query()->whereKey($identityId)->lockForUpdate()->first();
                if (! $identity instanceof Identity || $identity->disabled_at !== null || $identity->terminated_at !== null) {
                    throw new CharacterBootstrapIntentUnavailable('Canonical Platform Identity is unavailable for issuance.');
                }
                CharacterBootstrapIntentContract::assertUuid($identity->account_id, 'account_id');
                $revision = $databaseHighWater + 1;
                $issuedAt = now()->getTimestamp();
                $row = [
                    'source_revision' => $revision,
                    'operation_id' => $operationId,
                    'issuer_decision_id' => strtolower((string) Str::uuid()),
                    'issuer_authority' => $issuer,
                    'identity_id' => $identity->id,
                    'account_id' => $identity->account_id,
                    'target_world_id' => $targetWorldId,
                    ...$context,
                    'issued_at_source' => $issuedAt,
                    'expires_at_source' => $issuedAt + $ttl,
                    'created_at' => now(),
                ];
                DB::table('character_bootstrap_intents')->insert($row);
                DB::table('character_bootstrap_intent_authority_states')->where('id', 1)->update(['source_revision' => $revision, 'updated_at' => now()]);

                return $this->payload((object) $row);
            }, 3);
            $revision = $this->positiveDecimalInt($payload['source_revision'], 'source_revision');
            if ($revision > $floor) {
                $advance($revision);
            }

            return $payload;
        });
    }

    /** @return array<string, int|string|array<string, string>> */
    public function current(string $operationId): array
    {
        CharacterBootstrapIntentContract::assertUuid($operationId, 'operation_id');

        return $this->highWater->withLock(function (int $floor) use ($operationId): array {
            $databaseHighWater = $this->authorityRevision();
            if ($floor !== $databaseHighWater) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap source ordering is ambiguous.');
            }
            $row = DB::table('character_bootstrap_intents')->where('operation_id', $operationId)->first();
            if (! $row instanceof stdClass || $this->rowInt($row, 'source_revision') < 1 || $this->rowInt($row, 'source_revision') > $floor) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap intent is unavailable.');
            }
            if ($this->rowInt($row, 'issued_at_source') > now()->getTimestamp() || $this->rowInt($row, 'expires_at_source') <= now()->getTimestamp()) {
                throw new CharacterBootstrapIntentUnavailable('Character bootstrap intent is not current.');
            }

            return $this->payload($row);
        });
    }

    /**
     * @param  array{profile_revision: string, ruleset_revision: string, content_revision: string, starter_template_revision: string}  $context
     * @return array<string, int|string|array<string, string>>
     */
    private function reconcile(stdClass $row, int $identityId, string $targetWorldId, array $context): array
    {
        $same = $this->rowInt($row, 'identity_id') === $identityId && $this->rowString($row, 'target_world_id') === $targetWorldId;
        foreach ($context as $field => $value) {
            $same = $same && $this->rowString($row, $field) === $value;
        }
        if (! $same) {
            throw new CharacterBootstrapIntentConflict('Operation identity is already bound to different immutable semantics.');
        }

        return $this->payload($row);
    }

    /** @return array<string, int|string|array<string, string>> */
    private function payload(stdClass $row): array
    {
        return [
            'contract_version' => CharacterBootstrapIntentContract::VERSION,
            'variant' => CharacterBootstrapIntentContract::VARIANT,
            'issuer_authority' => $this->rowString($row, 'issuer_authority'),
            'issuer_decision_id' => $this->rowString($row, 'issuer_decision_id'),
            'source_revision' => $this->rowString($row, 'source_revision'),
            'operation_id' => $this->rowString($row, 'operation_id'),
            'operation' => CharacterBootstrapIntentContract::OPERATION,
            'account_id' => $this->rowString($row, 'account_id'),
            'target_world_id' => $this->rowString($row, 'target_world_id'),
            'interpretation_context' => [
                'profile_revision' => $this->rowString($row, 'profile_revision'),
                'ruleset_revision' => $this->rowString($row, 'ruleset_revision'),
                'content_revision' => $this->rowString($row, 'content_revision'),
                'starter_template_revision' => $this->rowString($row, 'starter_template_revision'),
            ],
            'issued_at_source' => $this->rowString($row, 'issued_at_source'),
            'expires_at_source' => $this->rowString($row, 'expires_at_source'),
            'audience' => CharacterBootstrapIntentContract::AUDIENCE,
        ];
    }

    private function authorityRevision(): int
    {
        $value = DB::table('character_bootstrap_intent_authority_states')->where('id', 1)->value('source_revision');

        return $value === null ? 0 : $this->positiveDecimalInt($value, 'source_revision', true);
    }

    private function rowString(stdClass $row, string $field): string
    {
        $value = $row->{$field} ?? null;
        if (! is_string($value) && ! is_int($value)) {
            throw new CharacterBootstrapIntentUnavailable("Stored {$field} is invalid.");
        }

        return (string) $value;
    }

    private function rowInt(stdClass $row, string $field): int
    {
        return $this->positiveDecimalInt($row->{$field} ?? null, $field);
    }

    private function positiveDecimalInt(mixed $value, string $field, bool $allowZero = false): int
    {
        if ((! is_int($value) && ! is_string($value)) || preg_match($allowZero ? '/^(0|[1-9][0-9]{0,18})$/' : '/^[1-9][0-9]{0,18}$/', (string) $value) !== 1) {
            throw new CharacterBootstrapIntentUnavailable("Stored {$field} is invalid.");
        }

        return (int) $value;
    }
}
