<?php

namespace Tests\Feature\ProductsEntitlements;

use App\Identity\Models\Identity;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use App\ProductsEntitlements\Premium\PremiumTimeEntitlement;
use App\ProductsEntitlements\Premium\PremiumTimeException;
use App\ProductsEntitlements\Premium\PremiumTimeLedger;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use Tests\TestCase;

final class PremiumTimeLedgerTest extends TestCase
{
    use RefreshDatabase;

    private const DAY = 86400;

    private const REASON = 'Operator grant for closed test cohort';

    protected function setUp(): void
    {
        parent::setUp();
        config(['hashing.driver' => 'bcrypt']);
        Carbon::setTestNow('2026-10-01T12:00:00Z');
    }

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        parent::tearDown();
    }

    public function test_first_grant_starts_interval_at_now_and_records_one_audit_event_without_reason_plaintext(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $now = now()->getTimestamp();

        $entitlement = $this->ledger()->grant($operator, $player, 30, self::REASON, $this->requestId());

        self::assertTrue(PremiumTimeContract::isUuidV7($entitlement->id));
        self::assertSame(1, $entitlement->lifecycleRevision);
        self::assertSame($now, $entitlement->effectiveFrom);
        self::assertSame($now + 30 * self::DAY, $entitlement->effectiveUntil);
        self::assertSame(PremiumTimeContract::STATE_ACTIVE, $entitlement->classify($now));

        $row = DB::table('premium_time_entitlements')->sole();
        self::assertSame(PremiumTimeContract::PRODUCT_ID, $row->product_id);
        self::assertEquals(1, $row->product_version);
        $event = DB::table('premium_time_entitlement_events')->sole();
        self::assertSame(PremiumTimeContract::ISSUANCE_OPERATOR_GRANT, $event->issuance_source);
        self::assertSame(hash('sha256', self::REASON), $event->reason_sha256);

        $audit = DB::table('admin_audit_events')->where('action', 'products.premium_granted')->sole();
        self::assertEquals($operator->id, $audit->actor_identity_id);
        self::assertSame((string) $player->id, $audit->target_id);
        self::assertIsString($audit->metadata);
        self::assertStringNotContainsString(self::REASON, $audit->metadata);
        $metadata = json_decode($audit->metadata, true, 3, JSON_THROW_ON_ERROR);
        self::assertIsArray($metadata);
        self::assertSame($player->account_id, $metadata['account_id']);
        self::assertSame($entitlement->id, $metadata['entitlement_id']);
        self::assertSame(1, $metadata['lifecycle_revision']);
    }

    public function test_grants_merge_into_one_interval_and_restart_after_expiry_with_the_same_entitlement(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $start = now()->getTimestamp();

        $first = $this->ledger()->grant($operator, $player, 10, self::REASON, $this->requestId());
        Carbon::setTestNow(Carbon::createFromTimestampUTC($start + 3 * self::DAY));
        $extended = $this->ledger()->grant($operator, $player, 5, self::REASON, $this->requestId());

        self::assertSame($first->id, $extended->id);
        self::assertSame(2, $extended->lifecycleRevision);
        self::assertSame($start, $extended->effectiveFrom);
        self::assertSame($start + 15 * self::DAY, $extended->effectiveUntil);

        // Exactly at effective_until the interval has ended; the next grant restarts at now.
        $expiredAt = $start + 15 * self::DAY;
        Carbon::setTestNow(Carbon::createFromTimestampUTC($expiredAt));
        self::assertSame(PremiumTimeContract::STATE_EXPIRED, $extended->classify($expiredAt));
        $restarted = $this->ledger()->grant($operator, $player, 1, self::REASON, $this->requestId());

        self::assertSame($first->id, $restarted->id);
        self::assertSame(3, $restarted->lifecycleRevision);
        self::assertSame($expiredAt, $restarted->effectiveFrom);
        self::assertSame($expiredAt + self::DAY, $restarted->effectiveUntil);
        self::assertSame(1, DB::table('premium_time_entitlements')->count());
        self::assertSame(3, DB::table('premium_time_entitlement_events')->count());
    }

    public function test_revoke_ends_the_interval_and_a_later_grant_starts_a_new_one_at_a_higher_revision(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $start = now()->getTimestamp();
        $granted = $this->ledger()->grant($operator, $player, 30, self::REASON, $this->requestId());

        Carbon::setTestNow(Carbon::createFromTimestampUTC($start + 2 * self::DAY));
        $revoked = $this->ledger()->revoke($operator, $player, 'Granted to the wrong account', $this->requestId());

        self::assertSame($granted->id, $revoked->id);
        self::assertSame(2, $revoked->lifecycleRevision);
        self::assertSame($start + 2 * self::DAY, $revoked->effectiveUntil);
        self::assertSame(PremiumTimeContract::STATE_REVOKED, $revoked->classify($start + 2 * self::DAY));
        // REVOKED wins even at a time inside the old interval.
        self::assertSame(PremiumTimeContract::STATE_REVOKED, $revoked->classify($start));
        self::assertSame(1, DB::table('admin_audit_events')->where('action', 'products.premium_revoked')->count());

        Carbon::setTestNow(Carbon::createFromTimestampUTC($start + 3 * self::DAY));
        $regranted = $this->ledger()->grant($operator, $player, 7, self::REASON, $this->requestId());
        self::assertSame(3, $regranted->lifecycleRevision);
        self::assertSame($start + 3 * self::DAY, $regranted->effectiveFrom);
        self::assertSame(PremiumTimeContract::STATE_ACTIVE, $regranted->classify($start + 3 * self::DAY));
    }

    public function test_revoke_is_refused_without_state_change_unless_the_entitlement_is_active(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];

        $this->assertRefused('premium_not_active', fn () => $this->ledger()->revoke($operator, $player, self::REASON, $this->requestId()));
        self::assertSame(0, DB::table('premium_time_entitlements')->count());

        $this->ledger()->grant($operator, $player, 1, self::REASON, $this->requestId());
        Carbon::setTestNow(Carbon::createFromTimestampUTC(now()->getTimestamp() + self::DAY));
        $this->assertRefused('premium_not_active', fn () => $this->ledger()->revoke($operator, $player, self::REASON, $this->requestId()));

        $this->ledger()->grant($operator, $player, 1, self::REASON, $this->requestId());
        $this->ledger()->revoke($operator, $player, self::REASON, $this->requestId());
        $this->assertRefused('premium_not_active', fn () => $this->ledger()->revoke($operator, $player, self::REASON, $this->requestId()));

        self::assertSame(3, $this->stored($player)->lifecycleRevision);
        self::assertSame(3, DB::table('premium_time_entitlement_events')->count());
        self::assertSame(3, DB::table('admin_audit_events')->count());
    }

    public function test_revoke_in_the_first_second_keeps_the_interval_ordered(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $start = now()->getTimestamp();

        $this->ledger()->grant($operator, $player, 1, self::REASON, $this->requestId());
        $revoked = $this->ledger()->revoke($operator, $player, self::REASON, $this->requestId());

        self::assertSame($start, $revoked->effectiveFrom);
        self::assertSame($start + 1, $revoked->effectiveUntil);
        self::assertSame(PremiumTimeContract::STATE_REVOKED, $revoked->classify($start));
    }

    public function test_exact_retry_is_idempotent_and_changed_reuse_of_a_request_id_conflicts(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];
        $other = $this->identity('other-player@example.test');
        $otherOperator = $this->identity('other-operator@example.test');
        $requestId = $this->requestId();

        $first = $this->ledger()->grant($operator, $player, 30, self::REASON, $requestId);
        Carbon::setTestNow(Carbon::createFromTimestampUTC(now()->getTimestamp() + 60));
        $retry = $this->ledger()->grant($operator, $player, 30, '  '.self::REASON.'  ', strtoupper($requestId));

        self::assertEquals($first, $retry);
        self::assertSame(1, DB::table('premium_time_entitlement_events')->count());
        self::assertSame(1, DB::table('admin_audit_events')->count());

        foreach ([
            fn () => $this->ledger()->grant($operator, $player, 31, self::REASON, $requestId),
            fn () => $this->ledger()->grant($operator, $player, 30, 'A different operator reason', $requestId),
            fn () => $this->ledger()->grant($operator, $other, 30, self::REASON, $requestId),
            fn () => $this->ledger()->grant($otherOperator, $player, 30, self::REASON, $requestId),
            fn () => $this->ledger()->revoke($operator, $player, self::REASON, $requestId),
        ] as $changed) {
            $this->assertRefused('premium_idempotency_conflict', $changed);
        }

        self::assertSame(1, $this->stored($player)->lifecycleRevision);
        self::assertNull(PremiumTimeEntitlement::forIdentity($other->id));
        self::assertSame(1, DB::table('premium_time_entitlement_events')->count());
    }

    public function test_duration_reason_request_and_horizon_bounds_are_refused_without_state_change(): void
    {
        [$operator, $player] = [$this->identity('operator@example.test'), $this->identity('player@example.test')];

        $this->assertRefused('premium_duration_invalid', fn () => $this->ledger()->grant($operator, $player, 0, self::REASON, $this->requestId()));
        $this->assertRefused('premium_duration_invalid', fn () => $this->ledger()->grant($operator, $player, 367, self::REASON, $this->requestId()));
        $this->assertRefused('premium_reason_invalid', fn () => $this->ledger()->grant($operator, $player, 1, 'too short', $this->requestId()));
        $this->assertRefused('premium_reason_invalid', fn () => $this->ledger()->grant($operator, $player, 1, str_repeat('x', 501), $this->requestId()));
        $this->assertRefused('premium_request_invalid', fn () => $this->ledger()->grant($operator, $player, 1, self::REASON, 'not-a-uuid'));
        self::assertSame(0, DB::table('premium_time_entitlements')->count());

        // Ten 366-day grants reach the 3660-day horizon exactly; one more day is refused as a whole.
        for ($i = 0; $i < 10; $i++) {
            $this->ledger()->grant($operator, $player, 366, self::REASON, $this->requestId());
        }
        $atCap = $this->stored($player);
        self::assertSame(now()->getTimestamp() + PremiumTimeContract::MAX_HORIZON_SECONDS, $atCap->effectiveUntil);
        $this->assertRefused('premium_horizon_exceeded', fn () => $this->ledger()->grant($operator, $player, 1, self::REASON, $this->requestId()));
        self::assertEquals($atCap, $this->stored($player));
        self::assertSame(10, DB::table('premium_time_entitlement_events')->count());
    }

    private function ledger(): PremiumTimeLedger
    {
        return app(PremiumTimeLedger::class);
    }

    private function stored(Identity $identity): PremiumTimeEntitlement
    {
        $stored = PremiumTimeEntitlement::forIdentity($identity->id);
        self::assertNotNull($stored);

        return $stored;
    }

    private function assertRefused(string $reason, callable $operation): void
    {
        try {
            $operation();
            self::fail("Expected premium time refusal {$reason}.");
        } catch (PremiumTimeException $exception) {
            self::assertSame($reason, $exception->reason);
        }
    }

    private function requestId(): string
    {
        return strtolower((string) Str::uuid());
    }

    private function identity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }
}
