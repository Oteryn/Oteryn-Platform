<?php

namespace App\ProductsEntitlements\Premium;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use Illuminate\Database\QueryException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;

/**
 * Operator grant and revocation of oteryn.premium_time v1 (contract 2.2, 2.3, 2.5). Route middleware enforces the
 * confirmed-MFA session and exact permission; this service owns interval merging, locking, idempotency and audit.
 */
final class PremiumTimeLedger
{
    public function __construct(
        private readonly PremiumTimeAuthority $authority,
        private readonly AdminAuditRecorder $audit,
    ) {}

    public function grant(Identity $actor, Identity $target, int $durationDays, string $reason, string $requestId): PremiumTimeEntitlement
    {
        if ($durationDays < PremiumTimeContract::MIN_GRANT_DAYS || $durationDays > PremiumTimeContract::MAX_GRANT_DAYS) {
            throw new PremiumTimeException('premium_duration_invalid', 'The grant must be between 1 and 366 days.');
        }

        return $this->mutate($actor, $target, PremiumTimeContract::EVENT_GRANT, $durationDays, $reason, $requestId);
    }

    public function revoke(Identity $actor, Identity $target, string $reason, string $requestId): PremiumTimeEntitlement
    {
        return $this->mutate($actor, $target, PremiumTimeContract::EVENT_REVOKE, null, $reason, $requestId);
    }

    private function mutate(Identity $actor, Identity $target, string $eventType, ?int $durationDays, string $reason, string $requestId): PremiumTimeEntitlement
    {
        $reason = trim($reason);
        if (mb_strlen($reason) < 10 || mb_strlen($reason) > 500) {
            throw new PremiumTimeException('premium_reason_invalid', 'Provide a reason between 10 and 500 characters.');
        }
        $requestId = strtolower($requestId);
        if (! PremiumTimeContract::isUuid($requestId)) {
            throw new PremiumTimeException('premium_request_invalid', 'The request identifier is invalid.');
        }
        $binding = [
            'identity_id' => $target->id,
            'actor_identity_id' => $actor->id,
            'event_type' => $eventType,
            'duration_days' => $durationDays,
            'reason_sha256' => hash('sha256', $reason),
        ];

        try {
            return DB::transaction(function () use ($actor, $target, $binding, $requestId): PremiumTimeEntitlement {
                $this->authority->lock($target->id);
                if ($this->replayed($requestId, $binding)) {
                    return $this->current($target->id);
                }

                $now = now()->getTimestamp();
                $existing = PremiumTimeEntitlement::forIdentity($target->id, true);
                $next = $binding['event_type'] === PremiumTimeContract::EVENT_GRANT
                    ? $this->granted($existing, (int) $binding['duration_days'], $now)
                    : $this->revoked($existing, $now);
                $this->store($existing, $next, $target->id);

                DB::table('premium_time_entitlement_events')->insert([
                    ...$binding,
                    'entitlement_id' => $next['id'],
                    'request_id' => $requestId,
                    'issuance_source' => PremiumTimeContract::ISSUANCE_OPERATOR_GRANT,
                    'lifecycle_revision' => $next['lifecycle_revision'],
                    'effective_from' => $next['effective_from'],
                    'effective_until' => $next['effective_until'],
                    'created_at' => now(),
                ]);

                $this->audit->record(
                    $actor->id,
                    $binding['event_type'] === PremiumTimeContract::EVENT_GRANT ? 'products.premium_granted' : 'products.premium_revoked',
                    'identity_premium_time',
                    (string) $target->id,
                    [
                        'account_id' => $target->account_id,
                        'entitlement_id' => $next['id'],
                        'lifecycle_revision' => $next['lifecycle_revision'],
                        'duration_days' => $binding['duration_days'],
                        'reason_sha256' => $binding['reason_sha256'],
                        'request_id' => $requestId,
                    ],
                );

                return $this->current($target->id);
            }, 3);
        } catch (QueryException $exception) {
            // A concurrent request for another target raced on the same request_id: judge it by the stored binding.
            if ($this->isDuplicateKey($exception) && $this->replayed($requestId, $binding)) {
                return $this->current($target->id);
            }

            throw new PremiumTimeException('premium_unavailable', 'Premium time is temporarily unavailable.');
        } catch (PremiumSnapshotUnavailable) {
            throw new PremiumTimeException('premium_unavailable', 'Premium time is temporarily unavailable.');
        }
    }

    /**
     * Contract 2.2: extend a running interval, otherwise start a new one at `$now`.
     *
     * @return array{id:string,state:string,lifecycle_revision:int,effective_from:int,effective_until:int}
     */
    private function granted(?PremiumTimeEntitlement $existing, int $durationDays, int $now): array
    {
        [$from, $base] = [$now, $now];
        if ($existing !== null && $existing->classify($now) === PremiumTimeContract::STATE_ACTIVE) {
            [$from, $base] = [$existing->effectiveFrom, $existing->effectiveUntil];
        }
        $until = $base + $durationDays * 86400;
        if ($until > $now + PremiumTimeContract::MAX_HORIZON_SECONDS) {
            throw new PremiumTimeException('premium_horizon_exceeded', 'The grant would end more than 3660 days from now.');
        }

        return [
            'id' => $existing->id ?? strtolower((string) Str::uuid7()),
            'state' => PremiumTimeContract::STORED_ACTIVE,
            'lifecycle_revision' => ($existing->lifecycleRevision ?? 0) + 1,
            'effective_from' => $from,
            'effective_until' => $until,
        ];
    }

    /** @return array{id:string,state:string,lifecycle_revision:int,effective_from:int,effective_until:int} */
    private function revoked(?PremiumTimeEntitlement $existing, int $now): array
    {
        if ($existing === null || $existing->classify($now) !== PremiumTimeContract::STATE_ACTIVE) {
            throw new PremiumTimeException('premium_not_active', 'This account has no active premium time to revoke.');
        }

        return [
            'id' => $existing->id,
            'state' => PremiumTimeContract::STORED_REVOKED,
            'lifecycle_revision' => $existing->lifecycleRevision + 1,
            'effective_from' => $existing->effectiveFrom,
            // Revoking in the first second of an interval keeps from < until.
            'effective_until' => max(min($existing->effectiveUntil, $now), $existing->effectiveFrom + 1),
        ];
    }

    /** @param array{id:string,state:string,lifecycle_revision:int,effective_from:int,effective_until:int} $next */
    private function store(?PremiumTimeEntitlement $existing, array $next, int $identityId): void
    {
        if ($next['lifecycle_revision'] > PremiumTimeContract::MAX_REVISION) {
            throw new PremiumTimeException('premium_unavailable', 'Premium time is temporarily unavailable.');
        }
        $values = [
            'state' => $next['state'],
            'lifecycle_revision' => $next['lifecycle_revision'],
            'effective_from' => $next['effective_from'],
            'effective_until' => $next['effective_until'],
            'updated_at' => now(),
        ];

        if ($existing === null) {
            DB::table('premium_time_entitlements')->insert([
                ...$values,
                'id' => $next['id'],
                'identity_id' => $identityId,
                'product_id' => PremiumTimeContract::PRODUCT_ID,
                'product_version' => PremiumTimeContract::PRODUCT_VERSION,
                'created_at' => now(),
            ]);

            return;
        }

        $updated = DB::table('premium_time_entitlements')
            ->where('id', $existing->id)
            ->where('lifecycle_revision', $existing->lifecycleRevision)
            ->update($values);
        if ($updated !== 1) {
            throw new PremiumTimeException('premium_unavailable', 'Premium time is temporarily unavailable.');
        }
    }

    /** @param array{identity_id:int,actor_identity_id:int,event_type:string,duration_days:?int,reason_sha256:string} $binding */
    private function replayed(string $requestId, array $binding): bool
    {
        $event = DB::table('premium_time_entitlement_events')->where('request_id', $requestId)->first();
        if ($event === null) {
            return false;
        }
        $duration = $event->duration_days === null ? null : PremiumTimeContract::databaseInteger($event->duration_days);
        $reasonHash = $event->reason_sha256;
        if (PremiumTimeContract::databaseInteger($event->identity_id) !== $binding['identity_id']
            || PremiumTimeContract::databaseInteger($event->actor_identity_id) !== $binding['actor_identity_id']
            || $event->event_type !== $binding['event_type']
            || $duration !== $binding['duration_days']
            || ! is_string($reasonHash)
            || ! hash_equals($reasonHash, $binding['reason_sha256'])) {
            throw new PremiumTimeException('premium_idempotency_conflict', 'The request identifier is already in use.');
        }

        return true;
    }

    private function current(int $identityId): PremiumTimeEntitlement
    {
        $current = PremiumTimeEntitlement::forIdentity($identityId);
        if ($current === null) {
            throw new PremiumTimeException('premium_unavailable', 'Premium time is temporarily unavailable.');
        }

        return $current;
    }

    private function isDuplicateKey(QueryException $exception): bool
    {
        $driverCode = $exception->errorInfo[1] ?? null;

        return (string) $exception->getCode() === '23000'
            && (is_int($driverCode) || is_string($driverCode))
            && in_array((int) $driverCode, [1062, 19], true);
    }
}
