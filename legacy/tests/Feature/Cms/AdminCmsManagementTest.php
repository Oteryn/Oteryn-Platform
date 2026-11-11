<?php

namespace Tests\Feature\Cms;

use App\Admin\AdminRoleManager;
use App\Cms\Models\ManagedPage;
use App\Cms\Models\NewsPost;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Tests\TestCase;

final class AdminCmsManagementTest extends TestCase
{
    use RefreshDatabase;

    public function test_content_editor_can_create_draft_then_publish_news_with_audit(): void
    {
        $actor = $this->createIdentity('news-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.news.store'), [
            'slug' => 'phase-six-news',
            'title' => 'Phase Six News',
            'body' => 'Draft body',
            'published_at' => null,
        ])->assertRedirect();

        $post = NewsPost::query()->where('slug', 'phase-six-news')->firstOrFail();
        $this->get(route('news.show', ['slug' => $post->slug]))->assertNotFound();
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'cms.news_created',
            'target_type' => 'news_post',
            'target_id' => (string) $post->id,
        ]);

        $publishedAt = now()->subMinute()->format('Y-m-d H:i:s');
        $this->put(route('admin.news.update', $post), [
            'slug' => 'phase-six-news',
            'title' => 'Phase Six News Published',
            'body' => 'Published body',
            'published_at' => $publishedAt,
        ])->assertRedirect(route('admin.news.edit', $post));

        $this->get(route('news.show', ['slug' => $post->slug]))
            ->assertOk()
            ->assertSeeText('Phase Six News Published')
            ->assertSeeText('Published body');
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'cms.news_updated',
            'target_type' => 'news_post',
            'target_id' => (string) $post->id,
        ]);
    }

    public function test_content_editor_can_unpublish_existing_news_and_hide_it_from_public_reads(): void
    {
        $actor = $this->createIdentity('news-unpublish-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $post = NewsPost::query()->create([
            'slug' => 'published-then-hidden',
            'title' => 'Published then hidden',
            'body' => 'Visible before unpublish',
            'published_at' => now()->subMinute(),
        ]);

        $this->get(route('news.show', ['slug' => $post->slug]))
            ->assertOk()
            ->assertSeeText('Published then hidden');
        $this->get(route('news.index'))
            ->assertOk()
            ->assertSeeText('Published then hidden');

        $this->put(route('admin.news.update', $post), [
            'slug' => $post->slug,
            'title' => 'Published then hidden',
            'body' => 'Visible before unpublish',
            'published_at' => null,
        ])->assertRedirect(route('admin.news.edit', $post));

        $post->refresh();
        self::assertNull($post->published_at);
        $this->get(route('news.show', ['slug' => $post->slug]))->assertNotFound();
        $this->get(route('news.index'))
            ->assertOk()
            ->assertDontSeeText('Published then hidden');
        $this->assertAuditMetadata([
            'actor_identity_id' => $actor->id,
            'action' => 'cms.news_updated',
            'target_type' => 'news_post',
            'target_id' => (string) $post->id,
        ], [
            'slug' => $post->slug,
            'published' => false,
        ]);
    }

    public function test_content_editor_can_manage_published_plain_text_page_with_escaped_output_and_audit(): void
    {
        $actor = $this->createIdentity('page-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.pages.store'), [
            'slug' => 'about-oteryn',
            'title' => '<script>alert("title")</script>',
            'body' => '<img src=x onerror=alert("body")>',
            'published_at' => now()->subMinute()->format('Y-m-d H:i:s'),
        ])->assertRedirect();

        $page = ManagedPage::query()->where('slug', 'about-oteryn')->firstOrFail();

        $this->get(route('pages.show', ['slug' => $page->slug]))
            ->assertOk()
            ->assertSee('&lt;script&gt;alert(&quot;title&quot;)&lt;/script&gt;', false)
            ->assertSee('&lt;img src=x onerror=alert(&quot;body&quot;)&gt;', false)
            ->assertDontSee('<script>', false)
            ->assertDontSee('<img src=x', false);

        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'cms.page_created',
            'target_type' => 'managed_page',
            'target_id' => (string) $page->id,
        ]);
    }

    public function test_content_editor_can_edit_and_unpublish_existing_managed_page(): void
    {
        $actor = $this->createIdentity('page-update-editor@example.com');
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $page = ManagedPage::query()->create([
            'slug' => 'editable-page',
            'title' => 'Original title',
            'body' => 'Original body',
            'published_at' => now()->subMinute(),
        ]);

        $this->get(route('pages.show', ['slug' => $page->slug]))
            ->assertOk()
            ->assertSeeText('Original title')
            ->assertSeeText('Original body');

        $this->put(route('admin.pages.update', $page), [
            'slug' => $page->slug,
            'title' => 'Updated title',
            'body' => 'Updated body',
            'published_at' => null,
        ])->assertRedirect(route('admin.pages.edit', $page));

        $page->refresh();
        self::assertSame('Updated title', $page->title);
        self::assertSame('Updated body', $page->body);
        self::assertNull($page->published_at);
        $this->get(route('pages.show', ['slug' => $page->slug]))->assertNotFound();
        $this->assertAuditMetadata([
            'actor_identity_id' => $actor->id,
            'action' => 'cms.page_updated',
            'target_type' => 'managed_page',
            'target_id' => (string) $page->id,
        ], [
            'slug' => $page->slug,
            'published' => false,
        ]);
    }

    public function test_public_managed_page_hides_drafts_and_future_scheduled_pages(): void
    {
        ManagedPage::query()->create([
            'slug' => 'draft-page',
            'title' => 'Draft',
            'body' => 'Draft body',
            'published_at' => null,
        ]);
        ManagedPage::query()->create([
            'slug' => 'future-page',
            'title' => 'Future',
            'body' => 'Future body',
            'published_at' => now()->addHour(),
        ]);

        $this->get(route('pages.show', ['slug' => 'draft-page']))->assertNotFound();
        $this->get(route('pages.show', ['slug' => 'future-page']))->assertNotFound();
    }

    public function test_security_admin_without_cms_permission_cannot_mutate_news_or_pages(): void
    {
        $actor = $this->createIdentity('security-only@example.com');
        $this->assignRole($actor, AdminRoleManager::SECURITY_ADMIN);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.news.store'), [
            'slug' => 'denied-news',
            'title' => 'Denied',
            'body' => 'Denied',
        ])->assertForbidden();

        $this->post(route('admin.pages.store'), [
            'slug' => 'denied-page',
            'title' => 'Denied',
            'body' => 'Denied',
        ])->assertForbidden();

        self::assertSame(0, NewsPost::query()->where('slug', 'denied-news')->count());
        self::assertSame(0, ManagedPage::query()->where('slug', 'denied-page')->count());
    }

    public function test_cms_permission_does_not_bypass_confirmed_mfa_requirement(): void
    {
        $actor = $this->createIdentity('editor-no-mfa@example.com', false);
        $this->assignRole($actor, AdminRoleManager::CONTENT_EDITOR);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.news.store'), [
            'slug' => 'mfa-denied',
            'title' => 'Denied',
            'body' => 'Denied',
        ])->assertForbidden();

        self::assertSame(0, NewsPost::query()->where('slug', 'mfa-denied')->count());
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

    private function assignRole(Identity $identity, string $roleKey): void
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

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }

    /**
     * The json metadata column has no equality operator on PostgreSQL, so compare it decoded.
     *
     * @param  array<string, mixed>  $columns
     * @param  array<string, mixed>  $metadata
     */
    private function assertAuditMetadata(array $columns, array $metadata): void
    {
        $this->assertDatabaseHas('admin_audit_events', $columns);
        $stored = DB::table('admin_audit_events')->where($columns)->pluck('metadata')
            ->map(fn (mixed $value): mixed => is_string($value) ? json_decode($value, true, flags: JSON_THROW_ON_ERROR) : $value)
            ->all();
        self::assertContains($metadata, $stored);
    }
}
