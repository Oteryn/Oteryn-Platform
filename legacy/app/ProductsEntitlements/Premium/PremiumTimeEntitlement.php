<?php

namespace App\ProductsEntitlements\Premium;

use Illuminate\Support\Facades\DB;

/** Validated read of one account's stored entitlement; invalid durable state is unavailable, never guessed. */
final readonly class PremiumTimeEntitlement
{
    private function __construct(
        public string $id,
        public string $storedState,
        public int $lifecycleRevision,
        public int $effectiveFrom,
        public int $effectiveUntil,
    ) {}

    public static function forIdentity(int $identityId, bool $lock = false): ?self
    {
        $query = DB::table('premium_time_entitlements')->where('identity_id', $identityId);
        $row = ($lock ? $query->lockForUpdate() : $query)->first();
        if ($row === null) {
            return null;
        }

        if ($row->product_id !== PremiumTimeContract::PRODUCT_ID
            || PremiumTimeContract::databaseInteger($row->product_version) !== PremiumTimeContract::PRODUCT_VERSION) {
            throw new PremiumSnapshotUnavailable('Stored premium time entitlement is invalid.');
        }

        return self::validated($row->id, $row->state, $row->lifecycle_revision, $row->effective_from, $row->effective_until);
    }

    /** The result a ledger event recorded, so an idempotent retry returns it rather than the current state (contract 2.2). */
    public static function recorded(object $event): self
    {
        $state = match ($event->event_type ?? null) {
            PremiumTimeContract::EVENT_GRANT => PremiumTimeContract::STORED_ACTIVE,
            PremiumTimeContract::EVENT_REVOKE => PremiumTimeContract::STORED_REVOKED,
            default => null,
        };

        return self::validated($event->entitlement_id ?? null, $state, $event->lifecycle_revision ?? null, $event->effective_from ?? null, $event->effective_until ?? null);
    }

    private static function validated(mixed $id, mixed $state, mixed $revision, mixed $from, mixed $until): self
    {
        $revision = PremiumTimeContract::databaseInteger($revision);
        $from = PremiumTimeContract::databaseInteger($from);
        $until = PremiumTimeContract::databaseInteger($until);
        if (! is_string($id) || ! PremiumTimeContract::isUuidV7($id)
            || ! is_string($state)
            || ! in_array($state, [PremiumTimeContract::STORED_ACTIVE, PremiumTimeContract::STORED_REVOKED], true)
            || $revision === null || $revision < 1 || $revision > PremiumTimeContract::MAX_REVISION
            || $from === null || $until === null || $from >= $until) {
            throw new PremiumSnapshotUnavailable('Stored premium time entitlement is invalid.');
        }

        return new self($id, $state, $revision, $from, $until);
    }

    /** Contract 5.3, evaluated at Platform time `$now`. */
    public function classify(int $now): string
    {
        return match (true) {
            $this->storedState === PremiumTimeContract::STORED_REVOKED => PremiumTimeContract::STATE_REVOKED,
            $now >= $this->effectiveUntil => PremiumTimeContract::STATE_EXPIRED,
            $now < $this->effectiveFrom => PremiumTimeContract::STATE_NOT_YET_EFFECTIVE,
            default => PremiumTimeContract::STATE_ACTIVE,
        };
    }
}
