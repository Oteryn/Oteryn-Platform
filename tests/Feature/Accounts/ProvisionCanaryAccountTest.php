<?php

namespace Tests\Feature\Accounts;

use App\Accounts\Actions\ProvisionCanaryAccount;
use App\Accounts\Contracts\CanaryAccountProvisioningGateway;
use App\Accounts\Exceptions\CanaryAccountProvisioningConflict;
use App\Accounts\Exceptions\CanaryAccountProvisioningUnavailable;
use App\Accounts\Models\IdentityCanaryAccount;
use App\Audit\SecurityEventRecorder;
use App\Identity\Models\Identity;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class ProvisionCanaryAccountTest extends TestCase
{
    use RefreshDatabase;

    public function test_successful_provisioning_is_idempotent_and_does_not_call_gateway_again_once_ready(): void
    {
        $identity = $this->createIdentity('idempotent@example.com');
        $this->createPendingBinding($identity, 'op'.str_repeat('a', 30), 1_800_000_001);

        $gateway = new class implements CanaryAccountProvisioningGateway
        {
            public int $calls = 0;

            public function provision(string $provisioningName, int $creationEpoch): int
            {
                $this->calls++;

                return 7001;
            }
        };
        $this->app->instance(CanaryAccountProvisioningGateway::class, $gateway);

        $action = $this->app->make(ProvisionCanaryAccount::class);
        $first = $action->execute($identity->id);
        $second = $action->execute($identity->id);

        self::assertTrue($first->isReady());
        self::assertTrue($second->isReady());
        self::assertSame(7001, $second->canary_account_id);
        self::assertSame(1, $gateway->calls);
        self::assertSame(1, $this->eventCount($identity->id, SecurityEventRecorder::CANARY_ACCOUNT_PROVISIONING_COMPLETED));
    }

    public function test_retry_after_dependency_failure_reuses_pending_intent_and_completes_binding(): void
    {
        $identity = $this->createIdentity('retry@example.com');
        $binding = $this->createPendingBinding($identity, 'op'.str_repeat('b', 30), 1_800_000_002);

        $gateway = new class implements CanaryAccountProvisioningGateway
        {
            public int $calls = 0;

            public function provision(string $provisioningName, int $creationEpoch): int
            {
                $this->calls++;

                if ($this->calls === 1) {
                    throw new CanaryAccountProvisioningUnavailable('dependency unavailable');
                }

                return 7002;
            }
        };
        $this->app->instance(CanaryAccountProvisioningGateway::class, $gateway);
        $action = $this->app->make(ProvisionCanaryAccount::class);

        try {
            $action->execute($identity->id);
            self::fail('The first provisioning attempt should fail.');
        } catch (CanaryAccountProvisioningUnavailable) {
            // Expected.
        }

        $pending = $binding->fresh();
        self::assertNotNull($pending);
        self::assertSame(IdentityCanaryAccount::STATUS_PENDING, $pending->status);
        self::assertSame(ProvisionCanaryAccount::FAILURE_DEPENDENCY_UNAVAILABLE, $pending->last_failure_code);
        self::assertNull($pending->canary_account_id);

        $ready = $action->execute($identity->id);

        self::assertTrue($ready->isReady());
        self::assertSame(7002, $ready->canary_account_id);
        self::assertSame('op'.str_repeat('b', 30), $ready->provisioning_name);
        self::assertSame(1_800_000_002, $ready->canary_creation_epoch);
        self::assertSame(2, $gateway->calls);
    }

    public function test_gateway_conflict_marks_binding_fail_closed(): void
    {
        $identity = $this->createIdentity('conflict@example.com');
        $this->createPendingBinding($identity, 'op'.str_repeat('c', 30), 1_800_000_003);

        $this->app->instance(CanaryAccountProvisioningGateway::class, new class implements CanaryAccountProvisioningGateway
        {
            public function provision(string $provisioningName, int $creationEpoch): int
            {
                throw new CanaryAccountProvisioningConflict('creation marker mismatch');
            }
        });

        $this->expectException(CanaryAccountProvisioningConflict::class);

        try {
            $this->app->make(ProvisionCanaryAccount::class)->execute($identity->id);
        } finally {
            $binding = IdentityCanaryAccount::query()->whereKey($identity->id)->firstOrFail();
            self::assertTrue($binding->isConflict());
            self::assertFalse($binding->isReady());
            self::assertSame(ProvisionCanaryAccount::FAILURE_BINDING_CONFLICT, $binding->last_failure_code);
            self::assertSame(1, $this->eventCount($identity->id, SecurityEventRecorder::CANARY_ACCOUNT_PROVISIONING_CONFLICT));
        }
    }

    public function test_database_constraint_prevents_two_identities_from_binding_same_canary_account(): void
    {
        $firstIdentity = $this->createIdentity('first-owner@example.com');
        $secondIdentity = $this->createIdentity('second-owner@example.com');
        $this->createPendingBinding($firstIdentity, 'op'.str_repeat('d', 30), 1_800_000_004);
        $this->createPendingBinding($secondIdentity, 'op'.str_repeat('e', 30), 1_800_000_005);

        $this->app->instance(CanaryAccountProvisioningGateway::class, new class implements CanaryAccountProvisioningGateway
        {
            public function provision(string $provisioningName, int $creationEpoch): int
            {
                return 7003;
            }
        });
        $action = $this->app->make(ProvisionCanaryAccount::class);

        self::assertTrue($action->execute($firstIdentity->id)->isReady());

        try {
            $action->execute($secondIdentity->id);
            self::fail('The second Identity must not bind the same Canary account.');
        } catch (CanaryAccountProvisioningConflict) {
            // Expected.
        }

        $firstBinding = IdentityCanaryAccount::query()->whereKey($firstIdentity->id)->firstOrFail();
        $secondBinding = IdentityCanaryAccount::query()->whereKey($secondIdentity->id)->firstOrFail();

        self::assertTrue($firstBinding->isReady());
        self::assertSame(7003, $firstBinding->canary_account_id);
        self::assertTrue($secondBinding->isConflict());
        self::assertFalse($secondBinding->isReady());
        self::assertNull($secondBinding->canary_account_id);
    }

    private function createIdentity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function createPendingBinding(Identity $identity, string $name, int $creationEpoch): IdentityCanaryAccount
    {
        return IdentityCanaryAccount::query()->create([
            'identity_id' => $identity->id,
            'provisioning_name' => $name,
            'canary_creation_epoch' => $creationEpoch,
            'status' => IdentityCanaryAccount::STATUS_PENDING,
        ]);
    }

    private function eventCount(int $identityId, string $eventType): int
    {
        return (int) DB::table('identity_security_events')
            ->where('identity_id', $identityId)
            ->where('event_type', $eventType)
            ->count();
    }
}
