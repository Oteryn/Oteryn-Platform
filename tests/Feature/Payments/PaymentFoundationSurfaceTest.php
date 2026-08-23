<?php

namespace Tests\Feature\Payments;

use App\Identity\Models\Identity;
use App\Payments\Actions\CreatePaymentOrder;
use App\Payments\Infrastructure\DeterministicTestPaymentProvider;
use App\Payments\Models\PaymentAttempt;
use App\Payments\Models\PaymentOrder;
use App\Payments\Models\PaymentProviderEvent;
use App\Payments\Models\PaymentReconciliationEntry;
use App\Payments\Models\PaymentReconciliationResolution;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use RuntimeException;
use Tests\TestCase;

final class PaymentFoundationSurfaceTest extends TestCase
{
    use RefreshDatabase;

    private const SECRET = 'payments-foundation-synthetic-secret';

    protected function setUp(): void
    {
        parent::setUp();

        config([
            'app.env' => 'testing',
            'payments.enabled' => true,
            'payments.provider' => DeterministicTestPaymentProvider::PROVIDER,
            'payments.provider_verified' => false,
            'payments.allowed_currencies' => ['PLN', 'EUR'],
            'payments.maximum_order_amount_minor' => 100_000_000,
            'payments.webhook.maximum_payload_bytes' => 32_768,
            'payments.webhook.signature_tolerance_seconds' => 300,
            'payments.webhook.test_secret' => self::SECRET,
            'payments.test_surface.amount_minor' => 1_234,
        ]);
    }

    public function test_customer_history_and_return_are_owner_scoped_localized_and_ignore_browser_claims(): void
    {
        $owner = $this->identity('payment-history-owner@example.com');
        $other = $this->identity('payment-history-other@example.com');
        $ownerOrder = app(CreatePaymentOrder::class)->execute(
            $owner,
            'PLN',
            1_234,
            (string) Str::uuid(),
        );
        $otherOrder = app(CreatePaymentOrder::class)->execute(
            $other,
            'EUR',
            5_678,
            (string) Str::uuid(),
        );

        $this->actingAs($owner, 'web');

        $this->get(route('payments.account.index'))
            ->assertOk()
            ->assertSeeText('Payments')
            ->assertSeeText($ownerOrder->public_id)
            ->assertDontSeeText($otherOrder->public_id);

        $this->get(route('payments.account.index', ['locale' => 'pl']))
            ->assertOk()
            ->assertSeeText('Płatności')
            ->assertSeeText($ownerOrder->public_id)
            ->assertDontSeeText($otherOrder->public_id);

        $this->get(route('payments.account.return', [
            'orderPublicId' => $ownerOrder->public_id,
            'status' => 'succeeded',
        ]))
            ->assertOk()
            ->assertSeeText('Pending')
            ->assertSeeText('does not prove settlement');

        self::assertSame(PaymentOrder::STATUS_PENDING, $ownerOrder->refresh()->status);

        $this->get(route('payments.account.return', [
            'orderPublicId' => $otherOrder->public_id,
        ]))->assertNotFound();
    }

    public function test_synthetic_checkout_is_idempotent_non_production_only_and_never_mutates_wallet(): void
    {
        $identity = $this->identity('payment-test-checkout@example.com');
        $this->actingAs($identity, 'web');
        $requestId = (string) Str::uuid();

        $response = $this->post(route('payments.account.test-checkout.store'), [
            'request_id' => $requestId,
            'currency' => 'PLN',
        ]);

        $order = PaymentOrder::query()
            ->where('idempotency_key', 'payments-test-order:'.$requestId)
            ->firstOrFail();
        $response->assertRedirect(route('payments.account.return', [
            'orderPublicId' => $order->public_id,
        ]));

        self::assertSame(PaymentOrder::STATUS_CHECKOUT_CREATED, $order->status);
        self::assertSame(1_234, $order->amount_minor);
        self::assertSame(1, PaymentOrder::query()->count());
        self::assertSame(1, PaymentAttempt::query()->count());
        self::assertSame(0, DB::table('wallet_ledger_entries')->count());

        $this->post(route('payments.account.test-checkout.store'), [
            'request_id' => $requestId,
            'currency' => 'PLN',
        ])->assertRedirect(route('payments.account.return', [
            'orderPublicId' => $order->public_id,
        ]));

        self::assertSame(1, PaymentOrder::query()->count());
        self::assertSame(1, PaymentAttempt::query()->count());
        self::assertSame(0, DB::table('wallet_ledger_entries')->count());

        config(['app.env' => 'production']);
        $productionRequestId = (string) Str::uuid();
        $this->post(route('payments.account.test-checkout.store'), [
            'request_id' => $productionRequestId,
            'currency' => 'PLN',
        ])
            ->assertRedirect(route('payments.account.index'))
            ->assertSessionHasErrors('payments');

        self::assertSame(1, PaymentOrder::query()->count());
        self::assertSame(1, PaymentAttempt::query()->count());
        self::assertSame(0, DB::table('wallet_ledger_entries')->count());

        $payload = '{"malformed":';
        $this->call(
            'POST',
            '/api/v1/payments/test/events',
            [],
            [],
            [],
            ['CONTENT_TYPE' => 'application/json', 'HTTP_ACCEPT' => 'application/json'],
            $payload,
        )->assertStatus(400)->assertJson(['reason' => 'test_surface_unavailable']);
    }

    public function test_signed_test_ingress_changes_payment_truth_and_return_page_only_reflects_persisted_state(): void
    {
        $identity = $this->identity('payment-signed-ingress@example.com');
        $order = app(CreatePaymentOrder::class)->execute(
            $identity,
            'PLN',
            1_234,
            (string) Str::uuid(),
        );
        $timestamp = now()->getTimestamp();
        $payload = json_encode([
            'id' => (string) Str::uuid(),
            'type' => 'payment.succeeded',
            'created' => $timestamp,
            'data' => [
                'order_public_id' => $order->public_id,
                'currency' => 'PLN',
                'amount_minor' => 1_234,
                'provider_object_reference' => null,
                'customer_email' => 'must-not-be-stored@example.test',
            ],
        ], JSON_THROW_ON_ERROR);
        $signature = DeterministicTestPaymentProvider::signature(
            self::SECRET,
            $timestamp,
            $payload,
        );

        $response = $this->call(
            'POST',
            '/api/v1/payments/test/events',
            [],
            [],
            [],
            [
                'CONTENT_TYPE' => 'application/json',
                'HTTP_ACCEPT' => 'application/json',
                'HTTP_X_OTERYN_TEST_TIMESTAMP' => (string) $timestamp,
                'HTTP_X_OTERYN_TEST_SIGNATURE' => $signature,
            ],
            $payload,
        );

        $response->assertStatus(202)->assertJson([
            'status' => PaymentProviderEvent::STATE_PROCESSED,
            'reconciliation_reason' => null,
        ]);
        self::assertSame(PaymentOrder::STATUS_SUCCEEDED, $order->refresh()->status);
        self::assertSame(1, PaymentProviderEvent::query()->count());
        self::assertSame(0, DB::table('wallet_ledger_entries')->count());

        $stored = PaymentProviderEvent::query()->firstOrFail();
        self::assertStringNotContainsString(
            'must-not-be-stored@example.test',
            json_encode($stored->getAttributes(), JSON_THROW_ON_ERROR),
        );

        $this->actingAs($identity, 'web');
        $this->get(route('payments.account.return', ['orderPublicId' => $order->public_id]))
            ->assertOk()
            ->assertSeeText('Succeeded');
    }

    public function test_reconciliation_admin_requires_exact_permission_and_mfa_and_records_append_only_resolution_audit(): void
    {
        $orderOwner = $this->identity('payment-reconciliation-owner@example.com');
        $order = app(CreatePaymentOrder::class)->execute(
            $orderOwner,
            'EUR',
            4_200,
            (string) Str::uuid(),
        );
        $entry = PaymentReconciliationEntry::query()->create([
            'payment_order_id' => $order->id,
            'payment_provider_event_id' => null,
            'issue_type' => 'ambiguous_checkout_creation',
            'state' => PaymentReconciliationEntry::STATE_OPEN,
            'metadata' => ['internal_marker' => 'must-not-render'],
            'created_at' => now(),
        ]);

        $withoutPermission = $this->identity('payment-admin-denied@example.com');
        $this->actingAs($withoutPermission, 'web');
        $this->get(route('admin.payments.reconciliation.index'))->assertForbidden();

        $withoutMfa = $this->identity('payment-admin-no-mfa@example.com', false);
        $this->grantPermissions($withoutMfa, ['payments.reconcile']);
        $this->actingAs($withoutMfa, 'web');
        $this->get(route('admin.payments.reconciliation.index'))->assertForbidden();

        $admin = $this->identity('payment-admin@example.com');
        $this->grantPermissions($admin, ['payments.reconcile']);
        $this->actingAs($admin, 'web');

        $this->get(route('admin.payments.reconciliation.index'))
            ->assertOk()
            ->assertSeeText('Payment reconciliation')
            ->assertSeeText('ambiguous_checkout_creation')
            ->assertDontSeeText('must-not-render');

        $this->get(route('admin.payments.reconciliation.index', ['locale' => 'pl']))
            ->assertOk()
            ->assertSeeText('Uzgadnianie płatności')
            ->assertDontSeeText('must-not-render');

        $requestId = (string) Str::uuid();
        $payload = [
            'request_id' => $requestId,
            'resolution_code' => PaymentReconciliationResolution::REVIEWED_NO_PAYMENT_STATE_CHANGE,
        ];

        $this->post(route('admin.payments.reconciliation.resolve', $entry), $payload)
            ->assertRedirect(route('admin.payments.reconciliation.index'));

        self::assertSame(PaymentReconciliationEntry::STATE_RESOLVED, $entry->refresh()->state);
        self::assertSame(PaymentOrder::STATUS_PENDING, $order->refresh()->status);
        self::assertSame(1, PaymentReconciliationResolution::query()->count());
        self::assertSame(0, DB::table('wallet_ledger_entries')->count());
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $admin->id,
            'action' => 'payments.reconciliation_resolved_without_payment_state_change',
            'target_id' => (string) $entry->id,
        ]);

        $this->post(route('admin.payments.reconciliation.resolve', $entry), $payload)
            ->assertRedirect(route('admin.payments.reconciliation.index'));
        self::assertSame(1, PaymentReconciliationResolution::query()->count());
        self::assertSame(1, DB::table('admin_audit_events')
            ->where('action', 'payments.reconciliation_resolved_without_payment_state_change')
            ->count());

        $this->post(route('admin.payments.reconciliation.resolve', $entry), [
            'request_id' => (string) Str::uuid(),
            'resolution_code' => PaymentReconciliationResolution::REVIEWED_NO_PAYMENT_STATE_CHANGE,
        ])->assertSessionHasErrors('payments');

        self::assertSame(1, PaymentReconciliationResolution::query()->count());
        self::assertSame(PaymentOrder::STATUS_PENDING, $order->refresh()->status);
    }

    private function identity(string $email, bool $withMfa = true): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($withMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    /** @param list<string> $permissions */
    private function grantPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'payments-role-'.$identity->id,
            'name' => 'Payments test role',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($permissions as $permission) {
            $permissionId = $this->integerDatabaseValue(
                DB::table('admin_permissions')->where('key', $permission)->value('id'),
                "permission {$permission}",
            );
            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $permissionId,
            ]);
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    private function integerDatabaseValue(mixed $value, string $description): int
    {
        if (is_int($value)) {
            return $value;
        }

        if (is_string($value) && ctype_digit($value)) {
            return (int) $value;
        }

        throw new RuntimeException("Expected an integer-compatible {$description} id.");
    }
}
