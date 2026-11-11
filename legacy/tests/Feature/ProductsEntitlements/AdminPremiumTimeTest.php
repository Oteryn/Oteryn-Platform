<?php

namespace Tests\Feature\ProductsEntitlements;

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\ProductsEntitlements\Premium\PremiumTimeContract;
use App\ProductsEntitlements\Premium\PremiumTimeEntitlement;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Str;
use RuntimeException;
use Tests\TestCase;

final class AdminPremiumTimeTest extends TestCase
{
    use RefreshDatabase;

    private const REASON = 'Closed test cohort Premium allocation';

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

    public function test_premium_admin_requires_login_confirmed_mfa_and_the_exact_permission(): void
    {
        $target = $this->identity('premium-target@example.com');

        $this->get(route('admin.premium.index'))->assertRedirect();
        $this->post(route('admin.premium.grant'), $this->grantInput($target, 30))->assertRedirect();

        $withoutMfa = $this->identity('premium-no-mfa@example.com', mfa: false);
        $this->grantPermissions($withoutMfa, ['products.premium.manage']);
        $this->actingAsCurrent($withoutMfa);
        $this->get(route('admin.premium.index'))->assertForbidden();
        $this->post(route('admin.premium.grant'), $this->grantInput($target, 30))->assertForbidden();

        $otherPermission = $this->identity('premium-other-permission@example.com');
        $this->grantPermissions($otherPermission, ['marketplace.manage']);
        $this->actingAsCurrent($otherPermission);
        $this->get(route('admin.premium.index'))->assertForbidden();
        $this->post(route('admin.premium.grant'), $this->grantInput($target, 30))->assertForbidden();
        $this->post(route('admin.premium.revoke'), $this->revokeInput($target))->assertForbidden();

        self::assertNull(PremiumTimeEntitlement::forIdentity($target->id));
        self::assertSame(0, DB::table('admin_audit_events')->count());
    }

    public function test_premium_admin_grants_extends_and_revokes_with_audit_and_idempotent_replay(): void
    {
        $target = $this->identity('premium-target@example.com');
        $operator = $this->operator();
        $now = now()->getTimestamp();

        $this->get(route('admin.premium.index', ['email' => $target->email]))
            ->assertOk()
            ->assertSee('Premium state')
            ->assertSee(PremiumTimeContract::STATE_NONE)
            ->assertSee('name="account_id" value="'.$target->account_id.'"', false)
            ->assertDontSee('Revoke Premium time');

        $grant = $this->grantInput($target, 30);
        $this->post(route('admin.premium.grant'), $grant)
            ->assertRedirect(route('admin.premium.index', ['email' => $target->email]))
            ->assertSessionHas('status', 'Premium time granted.');
        // An exact retry of the same form submission changes nothing.
        $this->post(route('admin.premium.grant'), $grant)->assertRedirect();
        $this->post(route('admin.premium.grant'), $this->grantInput($target, 10))->assertRedirect();

        $stored = PremiumTimeEntitlement::forIdentity($target->id);
        self::assertNotNull($stored);
        self::assertSame(2, $stored->lifecycleRevision);
        self::assertSame($now + 40 * 86400, $stored->effectiveUntil);

        $this->get(route('admin.premium.index', ['email' => $target->email]))
            ->assertOk()
            ->assertSee(PremiumTimeContract::STATE_ACTIVE)
            ->assertSee(PremiumTimeContract::formatTime($now + 40 * 86400))
            ->assertSee('Revoke Premium time')
            ->assertDontSee(self::REASON);

        $this->post(route('admin.premium.revoke'), $this->revokeInput($target))
            ->assertRedirect(route('admin.premium.index', ['email' => $target->email]))
            ->assertSessionHas('status', 'Premium time revoked.');
        $revoked = PremiumTimeEntitlement::forIdentity($target->id);
        self::assertNotNull($revoked);
        self::assertSame(PremiumTimeContract::STATE_REVOKED, $revoked->classify($now));

        self::assertSame(2, DB::table('admin_audit_events')->where('action', 'products.premium_granted')->where('actor_identity_id', $operator->id)->count());
        self::assertSame(1, DB::table('admin_audit_events')->where('action', 'products.premium_revoked')->where('target_id', (string) $target->id)->count());
    }

    public function test_premium_admin_refusals_leave_state_unchanged_and_report_the_reason(): void
    {
        $target = $this->identity('premium-target@example.com');
        $this->operator();

        $this->from(route('admin.premium.index', ['email' => $target->email]))
            ->post(route('admin.premium.grant'), $this->grantInput($target, 367))
            ->assertSessionHasErrors('duration_days');
        $this->post(route('admin.premium.grant'), [...$this->grantInput($target, 5), 'reason' => 'short'])
            ->assertSessionHasErrors('reason');
        $this->post(route('admin.premium.grant'), [...$this->grantInput($target, 5), 'account_id' => strtolower((string) Str::uuid7())])
            ->assertSessionHasErrors('account_id');
        $this->post(route('admin.premium.grant'), [...$this->grantInput($target, 5), 'account_id' => substr_replace($target->account_id, '4', 14, 1)])
            ->assertSessionHasErrors('account_id');
        $this->post(route('admin.premium.grant'), [...$this->grantInput($target, 5), 'account_id' => null, 'email' => $target->email])
            ->assertSessionHasErrors('account_id');
        $this->post(route('admin.premium.revoke'), $this->revokeInput($target))
            ->assertSessionHasErrors('premium');

        $grant = $this->grantInput($target, 5);
        $this->post(route('admin.premium.grant'), $grant)->assertSessionHasNoErrors();
        $this->post(route('admin.premium.grant'), [...$grant, 'duration_days' => 6])
            ->assertSessionHasErrors('premium');

        $stored = PremiumTimeEntitlement::forIdentity($target->id);
        self::assertNotNull($stored);
        self::assertSame(1, $stored->lifecycleRevision);
        self::assertSame(1, DB::table('premium_time_entitlement_events')->count());
    }

    public function test_premium_admin_mutation_stays_bound_to_the_reviewed_account_after_an_email_is_reassigned(): void
    {
        $reviewed = $this->identity('premium-reviewed@example.com');
        $this->operator();
        $this->get(route('admin.premium.index', ['email' => 'premium-reviewed@example.com']))->assertOk();
        $grant = $this->grantInput($reviewed, 7);

        // The reviewed account changes its email and another account takes the old address before the form is sent.
        $reviewed->forceFill(['email' => 'premium-renamed@example.com'])->save();
        $successor = $this->identity('premium-reviewed@example.com');

        $this->post(route('admin.premium.grant'), $grant)->assertSessionHasNoErrors();

        self::assertNotNull(PremiumTimeEntitlement::forIdentity($reviewed->id));
        self::assertNull(PremiumTimeEntitlement::forIdentity($successor->id));
    }

    public function test_premium_admin_navigation_is_shown_only_with_the_exact_permission(): void
    {
        $plainAdmin = $this->identity('premium-plain-admin@example.com');
        $this->grantPermissions($plainAdmin, ['admin.access']);
        $this->actingAsCurrent($plainAdmin);
        $this->get(route('admin.dashboard'))->assertOk()->assertDontSee(route('admin.premium.index'), false);

        $operator = $this->identity('premium-nav-operator@example.com');
        $this->grantPermissions($operator, ['admin.access', 'products.premium.manage']);
        $this->actingAsCurrent($operator);
        $this->get(route('admin.dashboard'))->assertOk()->assertSee(route('admin.premium.index'), false);
    }

    public function test_premium_permission_is_catalogued_and_granted_to_platform_admin_only(): void
    {
        self::assertContains(AdminPermission::MANAGE_PREMIUM_TIME, AdminPermission::all());
        $roleKeys = DB::table('admin_role_permissions')
            ->join('admin_permissions', 'admin_permissions.id', '=', 'admin_role_permissions.permission_id')
            ->join('admin_roles', 'admin_roles.id', '=', 'admin_role_permissions.role_id')
            ->where('admin_permissions.key', AdminPermission::MANAGE_PREMIUM_TIME)
            ->pluck('admin_roles.key')
            ->all();

        self::assertSame([AdminRoleManager::PLATFORM_ADMIN], $roleKeys);
    }

    /** @return array{account_id: string, duration_days: int, reason: string, request_id: string} */
    private function grantInput(Identity $target, int $days): array
    {
        return ['account_id' => $target->account_id, 'duration_days' => $days, 'reason' => self::REASON, 'request_id' => (string) Str::uuid()];
    }

    /** @return array{account_id: string, reason: string, request_id: string} */
    private function revokeInput(Identity $target): array
    {
        return ['account_id' => $target->account_id, 'reason' => 'Granted to the wrong account', 'request_id' => (string) Str::uuid()];
    }

    private function operator(): Identity
    {
        $operator = $this->identity('premium-operator@example.com');
        $this->grantPermissions($operator, ['products.premium.manage']);
        $this->actingAsCurrent($operator);

        return $operator;
    }

    private function identity(string $email, bool $mfa = true): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        if ($mfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    /** @param  list<string>  $permissions */
    private function grantPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'premium-role-'.$identity->id,
            'name' => 'Premium test role',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($permissions as $permission) {
            $permissionId = DB::table('admin_permissions')->where('key', $permission)->value('id');
            if (! is_int($permissionId) && ! (is_string($permissionId) && ctype_digit($permissionId))) {
                throw new RuntimeException("Permission {$permission} is missing.");
            }
            DB::table('admin_role_permissions')->insert(['role_id' => $roleId, 'permission_id' => (int) $permissionId]);
        }

        DB::table('identity_admin_roles')->insert(['identity_id' => $identity->id, 'role_id' => $roleId]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $current = Identity::query()->findOrFail($identity->id);
        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $current->web_session_generation]);
    }
}
