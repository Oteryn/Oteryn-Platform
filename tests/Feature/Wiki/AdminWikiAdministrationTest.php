<?php

namespace Tests\Feature\Wiki;

use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\Wiki\Infrastructure\Audit\WikiAuditAction;
use App\Wiki\Infrastructure\Models\WikiArticle;
use App\Wiki\Infrastructure\Models\WikiArticleTranslation;
use App\Wiki\Infrastructure\Models\WikiCategory;
use App\Wiki\Infrastructure\Models\WikiRevision;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\URL;
use RuntimeException;
use Tests\TestCase;

final class AdminWikiAdministrationTest extends TestCase
{
    use RefreshDatabase;

    public function test_admin_routes_require_confirmed_mfa_and_exact_permissions(): void
    {
        $this->get(route('admin.wiki.index'))->assertRedirect(route('identity.login.create'));

        $withoutMfa = $this->createIdentity('wiki-no-mfa@example.com', false);
        $this->grantPermissions($withoutMfa, ['wiki.access', 'wiki.articles.manage']);
        $this->actingAsCurrent($withoutMfa);
        $this->get(route('admin.wiki.index'))->assertForbidden();

        $accessOnly = $this->createIdentity('wiki-access-only@example.com');
        $this->grantPermissions($accessOnly, ['wiki.access']);
        $this->actingAsCurrent($accessOnly);
        $this->get(route('admin.wiki.index'))->assertOk();
        $this->get(route('admin.wiki.articles.create'))->assertForbidden();
        $this->get(route('admin.wiki.categories.create'))->assertForbidden();
    }

    public function test_editor_can_create_update_preview_and_assign_categories_without_body_audit_leakage(): void
    {
        $actor = $this->createIdentity('wiki-editor@example.com');
        $this->grantPermissions($actor, [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.categories.manage',
        ]);
        $this->actingAsCurrent($actor);

        $validPattern = 'pattern="[a-z0-9]+([._\\-][a-z0-9]+)*"';
        $this->get(route('admin.wiki.categories.create'))
            ->assertOk()
            ->assertSee($validPattern, false);
        $this->get(route('admin.wiki.articles.create'))
            ->assertOk()
            ->assertSee($validPattern, false);

        $this->post(route('admin.wiki.categories.store'), $this->categoryPayload())
            ->assertRedirect()
            ->assertSessionHasNoErrors();
        $category = WikiCategory::query()->firstOrFail();

        $this->post(route('admin.wiki.articles.store'), $this->articlePayload([$category->id]))
            ->assertRedirect()
            ->assertSessionHasNoErrors();
        $article = WikiArticle::query()->firstOrFail();

        self::assertTrue($article->is_featured);
        self::assertSame(7, $article->sort_order);
        self::assertSame(1, $article->lock_version);
        $this->assertDatabaseHas('wiki_article_category', [
            'article_id' => $article->id,
            'category_id' => $category->id,
            'sort_order' => 0,
        ]);
        $this->assertDatabaseHas('wiki_revisions', [
            'article_id' => $article->id,
            'locale' => 'en',
            'revision_number' => 1,
        ]);
        $this->assertDatabaseHas('admin_audit_events', [
            'action' => WikiAuditAction::ARTICLE_PRESENTATION_UPDATED,
            'target_type' => 'wiki_article',
            'target_id' => (string) $article->id,
        ]);

        $preview = URL::temporarySignedRoute(
            'admin.wiki.articles.preview',
            now()->addMinutes(5),
            ['article' => $article, 'locale' => 'en'],
        );
        $this->get($preview)
            ->assertOk()
            ->assertSeeText('Installation heading')
            ->assertSee('id="installation-heading"', false)
            ->assertDontSee('<script>', false);

        $expiredPreview = URL::temporarySignedRoute(
            'admin.wiki.articles.preview',
            now()->subMinute(),
            ['article' => $article, 'locale' => 'en'],
        );
        $this->get($expiredPreview)->assertForbidden();

        $originalVersion = $article->lock_version;
        $updatedPayload = $this->articlePayload([$category->id]);
        $updatedPayload['translations']['en']['title'] = 'Updated installation';
        $updatedPayload['lock_version'] = $originalVersion;

        $this->put(route('admin.wiki.articles.update', $article), $updatedPayload)
            ->assertRedirect()
            ->assertSessionHasNoErrors();
        $article->refresh();
        self::assertSame($originalVersion + 1, $article->lock_version);
        self::assertSame(
            'Updated installation',
            WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->where('locale', 'en')
                ->value('title'),
        );

        $stalePayload = $this->articlePayload([$category->id]);
        $stalePayload['lock_version'] = $originalVersion;
        $conflictResponse = $this->put(route('admin.wiki.articles.update', $article), $stalePayload)
            ->assertStatus(409);
        self::assertStringNotContainsString('<style', $conflictResponse->getContent());
        self::assertStringNotContainsString('style=', $conflictResponse->getContent());

        $invalidPayload = $this->articlePayload([$category->id]);
        $invalidPayload['lock_version'] = $article->lock_version;
        $invalidPayload['translations']['en']['source_markdown'] = '<script>alert(1)</script>';
        $this->from(route('admin.wiki.articles.edit', $article))
            ->put(route('admin.wiki.articles.update', $article), $invalidPayload)
            ->assertRedirect(route('admin.wiki.articles.edit', $article))
            ->assertSessionHasErrors('translations.en');

        $auditPayload = json_encode(
            DB::table('admin_audit_events')->where('target_type', 'wiki_article')->get()->all(),
            JSON_THROW_ON_ERROR,
        );
        self::assertStringNotContainsString('Install the approved Oteryn client', $auditPayload);
        self::assertStringNotContainsString('Updated installation', $auditPayload);
    }

    public function test_manage_and_publish_permissions_are_separate_and_public_visibility_tracks_lifecycle(): void
    {
        $actor = $this->createIdentity('wiki-publisher@example.com');
        $this->grantPermissions($actor, ['wiki.access', 'wiki.articles.manage']);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.wiki.articles.store'), $this->articlePayload())
            ->assertRedirect()
            ->assertSessionHasNoErrors();
        $article = WikiArticle::query()->firstOrFail();

        $this->post(route('admin.wiki.articles.submit-review', $article), [
            'lock_version' => $article->lock_version,
        ])->assertRedirect();
        $article->refresh();
        self::assertSame('in_review', $article->status->value);

        $this->post(route('admin.wiki.articles.publish', $article), [
            'lock_version' => $article->lock_version,
        ])->assertForbidden();

        $this->grantPermissionsToExistingRole($actor, ['wiki.publish']);
        $this->post(route('admin.wiki.articles.publish', $article), [
            'lock_version' => $article->lock_version,
        ])->assertRedirect();
        $article->refresh();
        self::assertSame('published', $article->status->value);

        $this->get(route('wiki.article', ['locale' => 'en', 'slug' => 'installation-guide']))
            ->assertOk()
            ->assertSeeText('Installation guide');
        $this->get(route('wiki.article', ['locale' => 'pl', 'slug' => 'poradnik-instalacji']))
            ->assertOk()
            ->assertSeeText('Poradnik instalacji');

        $this->post(route('admin.wiki.articles.unpublish', $article), [
            'lock_version' => $article->lock_version,
        ])->assertRedirect();
        $article->refresh();
        self::assertSame('draft', $article->status->value);
        $this->get(route('wiki.article', ['locale' => 'en', 'slug' => 'installation-guide']))
            ->assertNotFound();
    }

    public function test_revision_restore_is_append_only_and_category_cycles_fail_with_conflict(): void
    {
        $actor = $this->createIdentity('wiki-restorer@example.com');
        $this->grantPermissions($actor, [
            'wiki.access',
            'wiki.articles.manage',
            'wiki.categories.manage',
            'wiki.publish',
        ]);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.wiki.categories.store'), $this->categoryPayload('root', 'Root', 'root'))
            ->assertRedirect();
        $root = WikiCategory::query()->firstOrFail();
        $this->post(route('admin.wiki.categories.store'), $this->categoryPayload(
            'child',
            'Child',
            'child',
            $root->id,
        ))->assertRedirect();
        $child = WikiCategory::query()->where('key', 'child')->firstOrFail();

        $cyclePayload = $this->categoryPayload('root', 'Root', 'root', $child->id);
        $cyclePayload['lock_version'] = $root->lock_version;
        $this->put(route('admin.wiki.categories.update', $root), $cyclePayload)->assertStatus(409);

        $this->post(route('admin.wiki.articles.store'), $this->articlePayload([$root->id]))->assertRedirect();
        $article = WikiArticle::query()->firstOrFail();
        $firstEnglishRevision = WikiRevision::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->orderBy('revision_number')
            ->firstOrFail();
        $beforeCount = WikiRevision::query()->where('article_id', $article->id)->count();

        $update = $this->articlePayload([$root->id]);
        $update['translations']['en']['title'] = 'Temporary replacement';
        $update['lock_version'] = $article->lock_version;
        $this->put(route('admin.wiki.articles.update', $article), $update)->assertRedirect();
        $article->refresh();

        $this->post(route('admin.wiki.articles.revisions.restore', [$article, $firstEnglishRevision]), [
            'lock_version' => $article->lock_version,
            'change_note' => 'Restore approved baseline.',
        ])->assertRedirect();
        $article->refresh();

        self::assertSame($beforeCount + 3, WikiRevision::query()->where('article_id', $article->id)->count());
        $restored = WikiRevision::query()
            ->where('article_id', $article->id)
            ->where('locale', 'en')
            ->orderByDesc('revision_number')
            ->firstOrFail();
        self::assertSame($firstEnglishRevision->id, $restored->source_revision_id);
        self::assertSame('Installation guide', $restored->title);
        self::assertSame(
            'Installation guide',
            WikiArticleTranslation::query()
                ->where('article_id', $article->id)
                ->where('locale', 'en')
                ->value('title'),
        );
    }

    /**
     * @param  list<int>  $categoryIds
     * @return array{
     *     content_type: string,
     *     is_featured: string,
     *     sort_order: int,
     *     category_ids: list<int>,
     *     change_note: string,
     *     translations: array{
     *         en: array{title: string, slug: string, summary: string, source_markdown: string},
     *         pl: array{title: string, slug: string, summary: string, source_markdown: string}
     *     },
     *     lock_version?: int
     * }
     */
    private function articlePayload(array $categoryIds = []): array
    {
        return [
            'content_type' => 'guide',
            'is_featured' => '1',
            'sort_order' => 7,
            'category_ids' => $categoryIds,
            'change_note' => 'Editorial baseline.',
            'translations' => [
                'en' => [
                    'title' => 'Installation guide',
                    'slug' => 'installation-guide',
                    'summary' => 'Approved installation guidance.',
                    'source_markdown' => "# Installation heading\n\nInstall the approved Oteryn client.",
                ],
                'pl' => [
                    'title' => 'Poradnik instalacji',
                    'slug' => 'poradnik-instalacji',
                    'summary' => 'Zatwierdzony poradnik instalacji.',
                    'source_markdown' => "# Instalacja\n\nZainstaluj zatwierdzonego klienta Oteryn.",
                ],
            ],
        ];
    }

    /**
     * @return array{
     *     key: string,
     *     parent_id: int|null,
     *     sort_order: int,
     *     visible: string,
     *     translations: array{
     *         en: array{name: string, slug: string, description: string},
     *         pl: array{name: string, slug: string, description: string}
     *     },
     *     lock_version?: int
     * }
     */
    private function categoryPayload(
        string $key = 'getting-started',
        string $name = 'Getting Started',
        string $slug = 'getting-started',
        ?int $parentId = null,
    ): array {
        return [
            'key' => $key,
            'parent_id' => $parentId,
            'sort_order' => 1,
            'visible' => '1',
            'translations' => [
                'en' => [
                    'name' => $name,
                    'slug' => $slug,
                    'description' => 'Start here.',
                ],
                'pl' => [
                    'name' => 'PL '.$name,
                    'slug' => 'pl-'.$slug,
                    'description' => 'Zacznij tutaj.',
                ],
            ],
        ];
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

    /** @param list<string> $permissions */
    private function grantPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'wiki-role-'.$identity->id,
            'name' => 'Wiki test role',
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

    /** @param list<string> $permissions */
    private function grantPermissionsToExistingRole(Identity $identity, array $permissions): void
    {
        $roleId = $this->integerDatabaseValue(
            DB::table('identity_admin_roles')->where('identity_id', $identity->id)->value('role_id'),
            'administrator role',
        );

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

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
