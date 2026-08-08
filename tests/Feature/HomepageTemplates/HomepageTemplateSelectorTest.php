<?php

namespace Tests\Feature\HomepageTemplates;

use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class HomepageTemplateSelectorTest extends TestCase
{
    use RefreshDatabase;

    public function test_production_template_is_the_default_and_public_design_gallery_stays_removed(): void
    {
        $this->get('/')
            ->assertOk()
            ->assertSee('production-home-shell', false);

        $this->get('/design/home-v2')->assertNotFound();
    }

    public function test_selector_requires_authentication_mfa_and_the_exact_permission(): void
    {
        $this->get('/admin/portal/homepage')->assertRedirect('/login');

        $identity = $this->createIdentityWithConfirmedMfa();
        $this->actingAsCurrent($identity);

        $this->get('/admin/portal/homepage')->assertForbidden();
    }

    public function test_approved_preview_is_private_noindex_and_does_not_mutate_public_selection(): void
    {
        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);

        $response = $this->get('/admin/portal/homepage/preview/classic')
            ->assertOk()
            ->assertHeader('X-Robots-Tag', 'noindex, nofollow')
            ->assertSee('classic-home-shell', false);

        $cacheControl = (string) $response->headers->get('Cache-Control');
        self::assertStringContainsString('no-store', $cacheControl);
        self::assertStringContainsString('private', $cacheControl);

        $this->assertDatabaseHas('homepage_template_settings', [
            'id' => 1,
            'active_key' => 'production',
            'version' => 0,
        ]);
    }

    public function test_activation_is_versioned_audited_and_changes_only_to_a_registered_template(): void
    {
        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);

        $this->put('/admin/portal/homepage/active', [
            'template' => 'classic',
            'version' => 0,
        ])->assertRedirect('/admin/portal/homepage');

        $this->assertDatabaseHas('homepage_template_settings', [
            'id' => 1,
            'active_key' => 'classic',
            'previous_key' => 'production',
            'version' => 1,
            'updated_by_identity_id' => $identity->id,
        ]);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $identity->id,
            'action' => 'portal.homepage_template.activate',
            'target_type' => 'homepage_template_setting',
            'target_id' => '1',
        ]);

        $this->get('/')
            ->assertOk()
            ->assertSee('classic-home-shell', false);
    }

    public function test_unregistered_template_is_rejected_server_side(): void
    {
        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);

        $this->put('/admin/portal/homepage/active', [
            'template' => 'resources.views.secret',
            'version' => 0,
        ])->assertSessionHasErrors('template');

        $this->assertDatabaseHas('homepage_template_settings', [
            'id' => 1,
            'active_key' => 'production',
            'version' => 0,
        ]);
    }

    public function test_stale_activation_is_rejected_without_overwriting_newer_state(): void
    {
        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);

        $this->put('/admin/portal/homepage/active', [
            'template' => 'classic',
            'version' => 0,
        ])->assertRedirect('/admin/portal/homepage');

        $this->put('/admin/portal/homepage/active', [
            'template' => 'production',
            'version' => 0,
        ])
            ->assertRedirect('/admin/portal/homepage')
            ->assertSessionHas('error');

        $this->assertDatabaseHas('homepage_template_settings', [
            'id' => 1,
            'active_key' => 'classic',
            'version' => 1,
        ]);
    }

    public function test_rollback_swaps_the_approved_templates_and_is_audited(): void
    {
        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);

        $this->put('/admin/portal/homepage/active', [
            'template' => 'classic',
            'version' => 0,
        ]);

        $this->post('/admin/portal/homepage/rollback', [
            'version' => 1,
        ])->assertRedirect('/admin/portal/homepage');

        $this->assertDatabaseHas('homepage_template_settings', [
            'id' => 1,
            'active_key' => 'production',
            'previous_key' => 'classic',
            'version' => 2,
        ]);
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $identity->id,
            'action' => 'portal.homepage_template.rollback',
            'target_type' => 'homepage_template_setting',
            'target_id' => '1',
        ]);
    }

    public function test_removed_template_key_falls_back_publicly_and_warns_the_administrator(): void
    {
        DB::table('homepage_template_settings')->where('id', 1)->update([
            'active_key' => 'removed-template',
            'version' => 7,
        ]);

        $this->get('/')
            ->assertOk()
            ->assertSee('production-home-shell', false);

        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);

        $this->get('/admin/portal/homepage')
            ->assertOk()
            ->assertSeeText('The stored homepage template is no longer registered');
    }

    public function test_selector_has_polish_localization(): void
    {
        $identity = $this->authorizedIdentity();
        $this->actingAsCurrent($identity);
        app()->setLocale('pl');

        $this->get('/admin/portal/homepage')
            ->assertOk()
            ->assertSeeText('Szablon strony głównej');
    }

    private function authorizedIdentity(): Identity
    {
        $identity = $this->createIdentityWithConfirmedMfa();
        $roleId = $this->numericId('admin_roles', 'key', 'platform_admin');
        $permissionId = $this->numericId('admin_permissions', 'key', 'portal.settings.manage');

        DB::table('admin_role_permissions')->insertOrIgnore([
            'role_id' => $roleId,
            'permission_id' => $permissionId,
        ]);
        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);

        return $identity;
    }

    private function createIdentityWithConfirmedMfa(): Identity
    {
        $identity = Identity::query()->create([
            'email' => 'homepage-admin@example.com',
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
        $identity->forceFill([
            'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
            'two_factor_confirmed_at' => now(),
        ])->save();

        return $identity;
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }

    private function numericId(string $table, string $column, string $value): int
    {
        $id = DB::table($table)->where($column, $value)->value('id');

        if ((! is_int($id) && ! is_string($id)) || ! is_numeric($id)) {
            self::fail("Expected numeric id for {$table}.{$column}={$value}.");
        }

        return (int) $id;
    }
}
