<?php

namespace Tests\Feature\Support;

use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Cms\Editorial\EditorialPageKey;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\ManagedPageLegalVersion;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class EditorialSupportLegalTest extends TestCase
{
    use RefreshDatabase;

    public function test_every_typed_route_has_deterministic_missing_unpublished_and_published_behavior(): void
    {
        foreach (EditorialPageKey::cases() as $key) {
            $this->get(route($key->publicRouteName()))
                ->assertNotFound()
                ->assertSeeText('has not been configured');
        }

        foreach (EditorialPageKey::cases() as $key) {
            ManagedPage::query()->create([
                'slug' => $key->managedPageSlug(),
                'title' => 'Draft '.$key->label(),
                'body' => 'DRAFT-SECRET-'.$key->value,
                'published_at' => $key === EditorialPageKey::ServerInformation ? now()->addHour() : null,
            ]);

            $this->get(route($key->publicRouteName()))
                ->assertNotFound()
                ->assertSeeText('not currently published')
                ->assertDontSeeText('DRAFT-SECRET-'.$key->value);
        }

        foreach (EditorialPageKey::cases() as $key) {
            $page = ManagedPage::query()->where('slug', $key->managedPageSlug())->firstOrFail();
            $page->forceFill([
                'title' => '<script>'.$key->label().'</script>',
                'body' => '<img src=x onerror=alert("typed")> '.$key->value,
                'legal_version' => $key->isLegal() ? '1.0' : null,
                'legal_effective_date' => $key->isLegal() ? '2026-07-24' : null,
                'published_at' => now()->subMinute(),
            ])->save();

            $this->get(route($key->publicRouteName()))
                ->assertOk()
                ->assertSee('&lt;script&gt;', false)
                ->assertSee('&lt;img src=x onerror=alert(&quot;typed&quot;)&gt;', false)
                ->assertDontSee('<script>', false)
                ->assertDontSee('<img src=x', false);
        }
    }

    public function test_reserved_editorial_slugs_cannot_be_managed_or_read_through_generic_page_routes(): void
    {
        $page = ManagedPage::query()->create([
            'slug' => EditorialPageKey::Support->managedPageSlug(),
            'title' => 'Support',
            'body' => 'Published support content',
            'published_at' => now()->subMinute(),
        ]);

        $this->get(route('pages.show', ['slug' => $page->slug]))->assertNotFound();

        $actor = $this->createIdentity('generic-editor@example.com');
        $this->assignExistingRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->get(route('admin.pages.edit', $page))->assertNotFound();

        $this->post(route('admin.pages.store'), [
            'slug' => EditorialPageKey::Rules->managedPageSlug(),
            'title' => 'Bypass attempt',
            'body' => 'Bypass attempt',
            'published_at' => now()->subMinute()->format('Y-m-d H:i:s'),
        ])->assertSessionHasErrors('slug');

        $this->assertDatabaseMissing('managed_pages', [
            'slug' => EditorialPageKey::Rules->managedPageSlug(),
        ]);
    }

    public function test_support_admin_requires_exact_permission_and_confirmed_mfa_and_records_bounded_audit(): void
    {
        $cmsOnly = $this->createIdentity('cms-only@example.com');
        $this->grantExactPermission($cmsOnly, AdminPermission::MANAGE_PAGES);
        $this->actingAsCurrent($cmsOnly);

        $this->put(route('admin.support-content.update', ['editorialPageKey' => EditorialPageKey::Support->value]), [
            'title' => 'Denied',
            'body' => 'Denied',
        ])->assertForbidden();

        $noMfa = $this->createIdentity('support-no-mfa@example.com', false);
        $this->assignExistingRole($noMfa, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($noMfa);

        $this->put(route('admin.support-content.update', ['editorialPageKey' => EditorialPageKey::Support->value]), [
            'title' => 'Denied',
            'body' => 'Denied',
        ])->assertForbidden();

        $actor = $this->createIdentity('support-editor@example.com');
        $this->assignExistingRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $sensitiveBody = 'Contact player@example.com but never audit this body or MFA-SECRET-EXAMPLE.';

        $this->put(route('admin.support-content.update', ['editorialPageKey' => EditorialPageKey::Support->value]), [
            'title' => 'Support guidance',
            'body' => $sensitiveBody,
            'published_at' => now()->subMinute()->format('Y-m-d H:i:s'),
        ])->assertRedirect();

        $page = ManagedPage::query()->where('slug', EditorialPageKey::Support->managedPageSlug())->firstOrFail();
        $auditQuery = DB::table('admin_audit_events')
            ->where('target_type', 'managed_page')
            ->where('target_id', (string) $page->id);
        $auditAction = $auditQuery->value('action');
        $auditMetadata = $auditQuery->value('metadata');

        if (! is_string($auditAction) || ! is_string($auditMetadata)) {
            self::fail('Expected string audit action and metadata values.');
        }

        self::assertSame('support.content_created', $auditAction);
        self::assertStringNotContainsString('player@example.com', $auditMetadata);
        self::assertStringNotContainsString('MFA-SECRET-EXAMPLE', $auditMetadata);
        self::assertStringNotContainsString($sensitiveBody, $auditMetadata);
    }

    public function test_legal_versions_and_effective_dates_are_preserved_and_immutable_per_version(): void
    {
        $actor = $this->createIdentity('legal-editor@example.com');
        $this->assignExistingRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $route = route('admin.support-content.update', ['editorialPageKey' => EditorialPageKey::Terms->value]);
        $publishedAt = now()->subMinute()->format('Y-m-d H:i:s');

        $this->put($route, [
            'title' => 'Terms of Service',
            'body' => 'Version one meaning',
            'legal_version' => '1.0',
            'legal_effective_date' => '2026-07-01',
            'published_at' => $publishedAt,
        ])->assertRedirect();

        $this->put($route, [
            'title' => 'Terms of Service',
            'body' => 'Version two meaning',
            'legal_version' => '2.0',
            'legal_effective_date' => '2026-08-01',
            'published_at' => $publishedAt,
        ])->assertRedirect();

        $page = ManagedPage::query()->where('slug', EditorialPageKey::Terms->managedPageSlug())->firstOrFail();
        $versionOne = ManagedPageLegalVersion::query()
            ->where('managed_page_id', $page->id)
            ->where('version', '1.0')
            ->firstOrFail();
        $versionTwo = ManagedPageLegalVersion::query()
            ->where('managed_page_id', $page->id)
            ->where('version', '2.0')
            ->firstOrFail();

        self::assertSame('2026-07-01', $versionOne->effective_date->format('Y-m-d'));
        self::assertSame('Version one meaning', $versionOne->body);
        self::assertSame('2026-08-01', $versionTwo->effective_date->format('Y-m-d'));
        self::assertSame('Version two meaning', $versionTwo->body);

        $this->from(route('admin.support-content.edit', ['editorialPageKey' => EditorialPageKey::Terms->value]))
            ->put($route, [
                'title' => 'Terms of Service',
                'body' => 'Silently changed meaning',
                'legal_version' => '2.0',
                'legal_effective_date' => '2026-08-01',
                'published_at' => $publishedAt,
            ])
            ->assertRedirect()
            ->assertSessionHasErrors('legal_version');

        $page->refresh();
        self::assertSame('Version two meaning', $page->body);
        self::assertSame('2.0', $page->legal_version);
        self::assertSame('2026-08-01', $page->legal_effective_date?->format('Y-m-d'));
        self::assertSame(2, DB::table('managed_page_legal_versions')->where('managed_page_id', $page->id)->count());
    }

    public function test_support_links_render_only_from_approved_configuration(): void
    {
        ManagedPage::query()->create([
            'slug' => EditorialPageKey::Support->managedPageSlug(),
            'title' => 'Support',
            'body' => 'Use an approved support channel.',
            'published_at' => now()->subMinute(),
        ]);

        config()->set('support.discord_url', 'https://discord.gg/oteryn');
        config()->set('support.discord_hosts', ['discord.gg']);
        config()->set('support.contact_email', 'support@example.test');
        config()->set('support.support_url', 'https://help.example.test/oteryn');
        config()->set('support.allowed_hosts', ['help.example.test']);

        $this->get(route('support.index'))
            ->assertOk()
            ->assertSee('https://discord.gg/oteryn', false)
            ->assertSee('mailto:support@example.test', false)
            ->assertSee('https://help.example.test/oteryn', false);

        config()->set('support.discord_url', 'javascript:alert(1)');
        config()->set('support.contact_email', 'not-an-email');
        config()->set('support.support_url', 'https://help.example.test.evil.invalid/oteryn');

        $this->get(route('support.index'))
            ->assertOk()
            ->assertDontSee('javascript:alert(1)', false)
            ->assertDontSee('mailto:not-an-email', false)
            ->assertDontSee('help.example.test.evil.invalid', false);
    }

    public function test_report_a_bug_is_guidance_only_and_has_no_submission_route(): void
    {
        ManagedPage::query()->create([
            'slug' => EditorialPageKey::ReportABug->managedPageSlug(),
            'title' => 'Report a Bug',
            'body' => 'Describe reproducible steps through an approved channel.',
            'published_at' => now()->subMinute(),
        ]);

        $this->get(route('support.report-a-bug'))
            ->assertOk()
            ->assertSeeText('does not store a support ticket submission');

        $this->post('/support/report-a-bug', [
            'email' => 'player@example.test',
            'description' => 'Do not persist this.',
        ])->assertStatus(405);

        $this->assertDatabaseMissing('managed_pages', [
            'body' => 'Do not persist this.',
        ]);
    }

    private function createIdentity(string $email, bool $confirmedMfa = true): Identity
    {
        $identity = Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);

        if ($confirmedMfa) {
            $identity->forceFill([
                'two_factor_secret' => 'TEST-MFA-SECRET-NOT-REAL',
                'two_factor_confirmed_at' => now(),
            ])->save();
        }

        return $identity;
    }

    private function assignExistingRole(Identity $identity, string $roleKey): void
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        if (! is_int($roleId) && ! (is_string($roleId) && ctype_digit($roleId))) {
            self::fail('Expected an integer-compatible administrator role id.');
        }

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => (int) $roleId,
        ]);
    }

    private function grantExactPermission(Identity $identity, string $permissionKey): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'support_editor_'.$identity->id,
            'name' => 'Support editor '.$identity->id,
            'created_at' => $now,
            'updated_at' => $now,
        ]);
        $permissionId = DB::table('admin_permissions')->where('key', $permissionKey)->value('id');

        if (! is_int($permissionId) && ! (is_string($permissionId) && ctype_digit($permissionId))) {
            self::fail('Expected an integer-compatible administrator permission id.');
        }

        DB::table('admin_role_permissions')->insert([
            'role_id' => $roleId,
            'permission_id' => (int) $permissionId,
        ]);
        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
