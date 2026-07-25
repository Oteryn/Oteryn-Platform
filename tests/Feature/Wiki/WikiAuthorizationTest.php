<?php

namespace Tests\Feature\Wiki;

use App\Admin\AdminAuthorization;
use App\Admin\AdminPermission;
use App\Admin\AdminRoleManager;
use App\Identity\Models\Identity;
use App\Wiki\Application\WikiArticleService;
use App\Wiki\Application\WikiAuthorization;
use App\Wiki\Domain\WikiTranslationInput;
use Illuminate\Auth\Access\AuthorizationException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class WikiAuthorizationTest extends TestCase
{
    use RefreshDatabase;

    public function test_existing_platform_admin_bundle_does_not_receive_reserved_wiki_permissions_implicitly(): void
    {
        $identity = $this->createIdentity('existing-platform-admin@example.com');
        $this->assignExistingRole($identity, AdminRoleManager::PLATFORM_ADMIN);
        $authorization = $this->app->make(AdminAuthorization::class);

        foreach ([
            AdminPermission::WIKI_ACCESS,
            AdminPermission::MANAGE_WIKI_ARTICLES,
            AdminPermission::MANAGE_WIKI_CATEGORIES,
            AdminPermission::PUBLISH_WIKI,
            'wiki.*',
        ] as $permission) {
            self::assertFalse($authorization->allows($identity, $permission));
        }
    }

    public function test_exact_wiki_permission_does_not_grant_other_wiki_capabilities(): void
    {
        $identity = $this->createIdentity('wiki-article-editor@example.com');
        $this->assignCustomRole($identity, 'wiki_article_editor_test', [
            AdminPermission::MANAGE_WIKI_ARTICLES,
        ]);
        $authorization = $this->app->make(WikiAuthorization::class);

        $authorization->assertCanManageArticles($identity);

        foreach ([
            static fn () => $authorization->assertCanAccess($identity),
            static fn () => $authorization->assertCanManageCategories($identity),
            static fn () => $authorization->assertCanPublish($identity),
        ] as $assertion) {
            try {
                $assertion();
                self::fail('Expected missing exact Wiki permission to fail closed.');
            } catch (AuthorizationException $exception) {
                self::assertSame(
                    'This Identity does not have the required Wiki permission.',
                    $exception->getMessage(),
                );
            }
        }
    }

    public function test_article_write_without_exact_permission_is_denied_without_persistence(): void
    {
        $identity = $this->createIdentity('wiki-denied@example.com');
        $service = $this->app->make(WikiArticleService::class);

        try {
            $service->create($identity, 'guide', [
                new WikiTranslationInput(
                    'en',
                    'Denied article',
                    'denied-article',
                    'Denied summary',
                    '# Denied',
                ),
            ]);
            self::fail('Expected Wiki article management to require its exact permission.');
        } catch (AuthorizationException) {
            self::assertDatabaseCount('wiki_articles', 0);
            self::assertDatabaseCount('wiki_revisions', 0);
            self::assertDatabaseCount('admin_audit_events', 0);
        }
    }

    private function createIdentity(string $email): Identity
    {
        return Identity::query()->create([
            'email' => $email,
            'password' => Hash::make('Correct-Horse-9!Battery'),
        ]);
    }

    private function assignExistingRole(Identity $identity, string $roleKey): void
    {
        $roleId = DB::table('admin_roles')->where('key', $roleKey)->value('id');

        self::assertIsInt($roleId);

        DB::table('identity_admin_roles')->insert([
            'identity_id' => $identity->id,
            'role_id' => $roleId,
        ]);
    }

    /**
     * @param  list<string>  $permissions
     */
    private function assignCustomRole(Identity $identity, string $roleKey, array $permissions): void
    {
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => $roleKey,
            'name' => 'Wiki test role',
            'created_at' => now(),
            'updated_at' => now(),
        ]);

        foreach ($permissions as $permission) {
            $permissionId = DB::table('admin_permissions')->where('key', $permission)->value('id');
            self::assertIsInt($permissionId);

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
}
