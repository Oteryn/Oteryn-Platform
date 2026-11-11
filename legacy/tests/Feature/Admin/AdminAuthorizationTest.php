<?php

namespace Tests\Feature\Admin;

use App\Admin\AdminAuthorization;
use App\Admin\AdminPermission;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Route;
use Tests\TestCase;

final class AdminAuthorizationTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();

        Route::middleware(['web', 'auth', 'mfa.confirmed', 'admin.permission:audit.view'])
            ->get('/_test/admin-audit', static fn (): string => 'audit allowed');
    }

    public function test_admin_route_requires_authentication(): void
    {
        $this->get('/admin')->assertRedirect('/login');
    }

    public function test_admin_route_requires_confirmed_mfa_even_when_role_grants_access(): void
    {
        $identity = $this->createIdentity();
        $this->assignRole($identity, 'content_editor');

        $this->actingAsCurrent($identity);

        $this->get('/admin')->assertForbidden();
    }

    public function test_confirmed_mfa_without_admin_role_is_forbidden(): void
    {
        $identity = $this->createIdentityWithConfirmedMfa();

        $this->actingAsCurrent($identity);

        $this->get('/admin')->assertForbidden();
    }

    public function test_role_without_requested_permission_is_forbidden(): void
    {
        $identity = $this->createIdentityWithConfirmedMfa();
        $this->assignRole($identity, 'content_editor');

        $this->actingAsCurrent($identity);

        $this->get('/_test/admin-audit')->assertForbidden();
    }

    public function test_authorization_service_allows_explicit_role_permission(): void
    {
        $identity = $this->createIdentityWithConfirmedMfa();
        $this->assignRole($identity, 'content_editor');

        self::assertTrue(app(AdminAuthorization::class)->allows($identity, AdminPermission::ACCESS));
    }

    public function test_explicit_role_permission_with_confirmed_mfa_allows_access(): void
    {
        $identity = $this->createIdentityWithConfirmedMfa();
        $this->assignRole($identity, 'content_editor');

        $this->actingAsCurrent($identity);

        $this->get('/admin')
            ->assertOk()
            ->assertSeeText('Oteryn Admin');
    }

    public function test_unknown_permission_fails_closed(): void
    {
        Route::middleware(['web', 'auth', 'mfa.confirmed', 'admin.permission:admin.unknown'])
            ->get('/_test/admin-unknown', static fn (): string => 'should not be reachable');

        $identity = $this->createIdentityWithConfirmedMfa();
        $this->assignRole($identity, 'platform_admin');

        $this->actingAsCurrent($identity);

        $this->get('/_test/admin-unknown')->assertForbidden();
    }

    private function createIdentity(): Identity
    {
        return Identity::query()->create([
            'email' => 'admin-candidate@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function createIdentityWithConfirmedMfa(): Identity
    {
        $identity = $this->createIdentity();
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();

        return $identity;
    }

    private function assignRole(Identity $identity, string $roleKey): void
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if (! is_int($roleId) && ! is_string($roleId)) {
            self::fail('Expected an integer-compatible administrator role id.');
        }

        if (! is_numeric($roleId)) {
            self::fail('Expected a numeric administrator role id.');
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => (int) $roleId,
        ]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
