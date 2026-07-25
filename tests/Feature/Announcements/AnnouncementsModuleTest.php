<?php

namespace Tests\Feature\Announcements;

use App\Announcements\Models\SiteAnnouncement;
use App\Announcements\Queries\ActiveAnnouncementQuery;
use App\Announcements\Queries\AnnouncementTickerProvider;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\PublicPortal\PublicContentState;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use RuntimeException;
use Tests\TestCase;

final class AnnouncementsModuleTest extends TestCase
{
    use RefreshDatabase;

    public function test_active_query_uses_inclusive_start_and_exclusive_end_boundaries(): void
    {
        $at = CarbonImmutable::parse('2026-07-24 12:00:00', 'UTC');

        SiteAnnouncement::factory()->create([
            'title' => 'Starts exactly now',
            'starts_at' => $at,
            'ends_at' => $at->addHour(),
        ]);
        SiteAnnouncement::factory()->create([
            'title' => 'Ends exactly now',
            'starts_at' => $at->subHour(),
            'ends_at' => $at,
        ]);
        SiteAnnouncement::factory()->create([
            'title' => 'Future',
            'starts_at' => $at->addSecond(),
            'ends_at' => $at->addHour(),
        ]);
        SiteAnnouncement::factory()->create([
            'title' => 'Draft',
            'starts_at' => $at->subHour(),
            'ends_at' => $at->addHour(),
            'publication_state' => SiteAnnouncement::STATE_DRAFT,
        ]);

        $titles = app(ActiveAnnouncementQuery::class)->active(5, $at)->pluck('title')->all();

        self::assertSame(['Starts exactly now'], $titles);
    }

    public function test_ticker_provider_exposes_explicit_state_and_escapes_plain_text_content(): void
    {
        $at = CarbonImmutable::parse('2026-07-24 12:00:00', 'UTC');
        SiteAnnouncement::factory()->create([
            'title' => '<script>alert("title")</script>',
            'body' => '<img src=x onerror=alert("body")>',
            'starts_at' => $at->subMinute(),
            'ends_at' => $at->addMinute(),
            'action_label' => 'Read',
            'action_url' => '/news',
        ]);

        $provider = app(AnnouncementTickerProvider::class);
        self::assertSame(PublicContentState::AVAILABLE, $provider->get($at)->state);

        $html = $provider->render($at)->render();
        self::assertStringContainsString('&lt;script&gt;', $html);
        self::assertStringContainsString('&lt;img src=x', $html);
        self::assertStringNotContainsString('<script>', $html);
        self::assertStringNotContainsString('<img src=x', $html);

        $empty = $provider->get($at->addDay());
        self::assertSame(PublicContentState::EMPTY, $empty->state);
        self::assertSame([], $empty->items);
    }

    public function test_admin_mutation_requires_exact_permission_audits_and_rejects_unsafe_links(): void
    {
        $actor = $this->createIdentity('announcement-editor@example.com');
        $this->actingAsCurrent($actor);
        $payload = $this->payload();

        $this->post(route('admin.announcements.store'), $payload)->assertForbidden();

        $this->grantPermissions($actor, ['portal.announcements.manage']);
        $this->post(route('admin.announcements.store'), [
            ...$payload,
            'action_label' => 'Unsafe',
            'action_url' => 'javascript:alert(1)',
        ])->assertSessionHasErrors('action_url');

        $this->post(route('admin.announcements.store'), [
            ...$payload,
            'action_label' => 'Community',
            'action_url' => 'https://example.org/community',
        ])->assertRedirect();

        $announcement = SiteAnnouncement::query()->firstOrFail();
        $this->assertDatabaseHas('admin_audit_events', [
            'actor_identity_id' => $actor->id,
            'action' => 'portal.announcement_created',
            'target_type' => 'site_announcement',
            'target_id' => (string) $announcement->id,
        ]);

        $auditPayload = DB::table('admin_audit_events')
            ->where('target_type', 'site_announcement')
            ->value('metadata');

        if (! is_string($auditPayload)) {
            throw new RuntimeException('Expected announcement audit metadata to be a string.');
        }

        self::assertStringNotContainsString($announcement->title, $auditPayload);
        self::assertStringNotContainsString($announcement->body, $auditPayload);
        self::assertStringNotContainsString((string) $announcement->action_url, $auditPayload);
    }

    public function test_stale_edit_fails_with_conflict_and_mfa_is_not_bypassed(): void
    {
        $actor = $this->createIdentity('announcement-stale@example.com');
        $this->grantPermissions($actor, ['portal.announcements.manage']);
        $this->actingAsCurrent($actor);

        $announcement = SiteAnnouncement::factory()->create([
            'title' => 'Original',
            'created_by' => $actor->id,
            'updated_by' => $actor->id,
        ]);
        $staleVersion = $announcement->lock_version;

        $this->put(route('admin.announcements.update', $announcement), [
            ...$this->payload(),
            'title' => 'Newer edit',
            'lock_version' => $staleVersion,
        ])->assertRedirect();
        $this->put(route('admin.announcements.update', $announcement), [
            ...$this->payload(),
            'title' => 'Stale edit',
            'lock_version' => $staleVersion,
        ])->assertStatus(409);
        self::assertSame('Newer edit', $announcement->refresh()->title);

        $withoutMfa = $this->createIdentity('announcement-no-mfa@example.com', false);
        $this->grantPermissions($withoutMfa, ['portal.announcements.manage']);
        $this->actingAsCurrent($withoutMfa);
        $this->post(route('admin.announcements.store'), $this->payload())->assertForbidden();
    }

    /**
     * @return array<string, mixed>
     */
    private function payload(): array
    {
        return [
            'title' => 'Maintenance notice',
            'body' => 'The service window is scheduled.',
            'severity' => SiteAnnouncement::SEVERITY_MAINTENANCE,
            'starts_at' => '2026-07-24T12:00',
            'ends_at' => '2026-07-24T13:00',
            'publication_state' => SiteAnnouncement::STATE_PUBLISHED,
            'action_label' => 'Details',
            'action_url' => '/news',
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

    /**
     * @param  list<string>  $permissions
     */
    private function grantPermissions(Identity $identity, array $permissions): void
    {
        $now = now();
        $roleId = DB::table('admin_roles')->insertGetId([
            'key' => 'announcement-role-'.$identity->id,
            'name' => 'Announcement test role',
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

    private function actingAsCurrent(Identity $identity): void
    {
        $currentIdentity = Identity::query()->findOrFail($identity->id);

        $this->actingAs($identity, 'web')
            ->withSession([WebSessionState::GENERATION_KEY => $currentIdentity->web_session_generation]);
    }
}
