<?php

namespace Tests\Feature\Events;

use App\Cms\Models\NewsPost;
use App\Events\Models\Event;
use App\Events\Models\EventTranslation;
use App\Events\Queries\EventCalendarQuery;
use App\Events\Queries\UpcomingEventProvider;
use App\Identity\Models\Identity;
use App\Identity\Sessions\WebSessionState;
use App\PublicPortal\PublicContentState;
use Carbon\CarbonImmutable;
use Illuminate\Database\QueryException;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Carbon;
use Illuminate\Support\Facades\App;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use RuntimeException;
use Tests\TestCase;

final class EventsModuleTest extends TestCase
{
    use RefreshDatabase;

    protected function tearDown(): void
    {
        Carbon::setTestNow();
        CarbonImmutable::setTestNow();
        App::setLocale('en');
        parent::tearDown();
    }

    public function test_public_calendar_classifies_active_upcoming_archived_cancelled_and_empty_states(): void
    {
        $at = CarbonImmutable::parse('2026-07-24 12:00:00', 'UTC');
        $this->freezeAt($at);

        $this->createEvent('active-event', Event::STATUS_SCHEDULED, $at->subHour(), $at->addHour(), 'Active event');
        $this->createEvent('upcoming-event', Event::STATUS_SCHEDULED, $at->addHour(), $at->addHours(2), 'Upcoming event');
        $this->createEvent('archived-event', Event::STATUS_ACTIVE, $at->subHours(2), $at, 'Archived event');
        $this->createEvent('cancelled-event', Event::STATUS_CANCELLED, $at->addDay(), $at->addDay()->addHour(), 'Cancelled event');
        $this->createEvent('draft-event', Event::STATUS_DRAFT, $at->addDay(), $at->addDay()->addHour(), 'Draft event');

        $this->get(route('events.index'))
            ->assertOk()
            ->assertSeeText('Active event')
            ->assertSeeText('Upcoming event')
            ->assertSeeText('Archived event')
            ->assertSeeText('Cancelled event')
            ->assertDontSeeText('Draft event');

        EventTranslation::query()->delete();
        Event::query()->delete();
        $this->get(route('events.index'))->assertOk()->assertSeeText('No events are available.');
    }

    public function test_public_detail_is_locale_specific_escaped_and_links_only_published_news(): void
    {
        $at = CarbonImmutable::parse('2026-07-24 12:00:00', 'UTC');
        $this->freezeAt($at);
        $news = NewsPost::query()->create([
            'slug' => 'related-news',
            'title' => 'Related news',
            'body' => 'News body',
            'published_at' => $at->subMinute(),
        ]);
        $event = Event::factory()->create([
            'status' => Event::STATUS_SCHEDULED,
            'starts_at' => $at->addHour(),
            'ends_at' => $at->addHours(2),
            'news_post_id' => $news->id,
        ]);
        EventTranslation::factory()->create([
            'event_id' => $event->id,
            'locale' => 'en',
            'slug' => 'english-slug',
            'title' => '<script>alert("title")</script>',
            'summary' => 'English summary',
            'body' => '<img src=x onerror=alert("body")>',
        ]);
        EventTranslation::factory()->create([
            'event_id' => $event->id,
            'locale' => 'pl',
            'slug' => 'polski-slug',
            'title' => 'Polski tytuł',
            'summary' => 'Polskie podsumowanie',
            'body' => 'Polska treść',
        ]);

        $this->get(route('events.show', ['slug' => 'english-slug']))
            ->assertOk()
            ->assertSee('&lt;script&gt;', false)
            ->assertSee('&lt;img src=x', false)
            ->assertDontSee('<script>', false)
            ->assertSeeText('Related news');

        App::setLocale('pl');
        $this->get(route('events.show', ['slug' => 'polski-slug']))->assertOk()->assertSeeText('Polski tytuł');
        $this->get(route('events.show', ['slug' => 'english-slug']))->assertNotFound();
    }

    public function test_localized_slug_uniqueness_is_enforced_per_locale(): void
    {
        $first = Event::factory()->create();
        $second = Event::factory()->create();

        EventTranslation::factory()->create([
            'event_id' => $first->id,
            'locale' => 'en',
            'slug' => 'shared-slug',
        ]);
        EventTranslation::factory()->create([
            'event_id' => $second->id,
            'locale' => 'pl',
            'slug' => 'shared-slug',
        ]);

        $this->expectException(QueryException::class);
        EventTranslation::factory()->create([
            'event_id' => $second->id,
            'locale' => 'en',
            'slug' => 'shared-slug',
        ]);
    }

    public function test_boundaries_and_upcoming_provider_are_deterministic_in_utc(): void
    {
        self::assertSame('UTC', config('app.timezone'));
        $start = CarbonImmutable::parse('2026-10-25 01:00:00', 'UTC');
        $end = CarbonImmutable::parse('2026-10-25 02:00:00', 'UTC');
        $this->createEvent('utc-boundary', Event::STATUS_SCHEDULED, $start, $end, 'UTC boundary');

        $query = app(EventCalendarQuery::class);
        self::assertSame(Event::STATUS_ACTIVE, $query->calendar('en', $start)['active'][0]['status']);
        self::assertSame(Event::STATUS_COMPLETED, $query->calendar('en', $end)['archived'][0]['status']);

        $provider = app(UpcomingEventProvider::class);
        $active = $provider->get('en', $start);
        self::assertSame(PublicContentState::AVAILABLE, $active->state);
        self::assertSame('UTC boundary', $active->event['title'] ?? null);
        self::assertSame(PublicContentState::EMPTY, $provider->get('en', $end)->state);
    }

    public function test_admin_workflow_separates_manage_and_publish_permissions_audits_and_rejects_stale_edits(): void
    {
        $at = CarbonImmutable::parse('2026-07-24 12:00:00', 'UTC');
        $this->freezeAt($at);
        $actor = $this->createIdentity('event-editor@example.com');
        $this->grantPermissions($actor, ['events.manage']);
        $this->actingAsCurrent($actor);

        $this->post(route('admin.events.store'), $this->eventPayload())->assertRedirect();
        $event = Event::query()->firstOrFail();
        self::assertSame(Event::STATUS_DRAFT, $event->status);
        self::assertSame('2026-07-25 12:00:00', $event->starts_at->utc()->toDateTimeString());

        $this->put(route('admin.events.status', $event), [
            'status' => Event::STATUS_SCHEDULED,
            'lock_version' => $event->lock_version,
        ])->assertForbidden();
        $this->grantPermissionsToExistingRole($actor, ['events.publish']);

        $this->put(route('admin.events.status', $event), [
            'status' => Event::STATUS_ACTIVE,
            'lock_version' => $event->lock_version,
        ])->assertStatus(409);
        $this->put(route('admin.events.status', $event), [
            'status' => Event::STATUS_SCHEDULED,
            'lock_version' => $event->lock_version,
        ])->assertRedirect();

        $event->refresh();
        $publishedVersion = $event->lock_version;
        $this->get(route('events.show', ['slug' => 'summer-tournament']))->assertOk();

        $this->put(route('admin.events.update', $event), [
            ...$this->eventPayload(),
            'translations' => [
                'en' => [
                    'title' => 'Updated event',
                    'slug' => 'summer-tournament',
                    'summary' => 'Updated summary',
                    'body' => 'Updated body',
                ],
            ],
            'lock_version' => $publishedVersion,
        ])->assertRedirect();

        $event->refresh();
        self::assertSame(Event::STATUS_DRAFT, $event->status);
        $this->get(route('events.show', ['slug' => 'summer-tournament']))->assertNotFound();
        $this->put(route('admin.events.update', $event), [
            ...$this->eventPayload(),
            'lock_version' => $publishedVersion,
        ])->assertStatus(409);
        $this->put(route('admin.events.status', $event), [
            'status' => Event::STATUS_SCHEDULED,
            'lock_version' => $publishedVersion,
        ])->assertStatus(409);

        $auditPayload = json_encode(
            DB::table('admin_audit_events')->where('target_type', 'event')->get()->all(),
            JSON_THROW_ON_ERROR,
        );
        self::assertStringNotContainsString('Summer tournament body', $auditPayload);
        self::assertStringNotContainsString('Updated event', $auditPayload);
        $this->assertDatabaseHas('admin_audit_events', [
            'action' => 'events.status_changed',
            'target_type' => 'event',
            'target_id' => (string) $event->id,
        ]);
    }

    public function test_admin_routes_require_confirmed_mfa_and_both_exact_permissions(): void
    {
        $actor = $this->createIdentity('event-no-mfa@example.com', false);
        $this->grantPermissions($actor, ['events.manage', 'events.publish']);
        $this->actingAsCurrent($actor);
        $this->post(route('admin.events.store'), $this->eventPayload())->assertForbidden();

        $publisherOnly = $this->createIdentity('publisher-only@example.com');
        $this->grantPermissions($publisherOnly, ['events.publish']);
        $this->actingAsCurrent($publisherOnly);
        $event = $this->createEvent(
            'publisher-only-target',
            Event::STATUS_DRAFT,
            now()->addDay()->toImmutable(),
            now()->addDay()->addHour()->toImmutable(),
            'Publisher only target',
        );
        $this->put(route('admin.events.status', $event), [
            'status' => Event::STATUS_SCHEDULED,
            'lock_version' => $event->lock_version,
        ])->assertForbidden();
    }

    /**
     * @return array<string, mixed>
     */
    private function eventPayload(): array
    {
        return [
            'starts_at' => '2026-07-25T12:00',
            'ends_at' => '2026-07-25T14:00',
            'featured' => '1',
            'news_post_id' => null,
            'translations' => [
                'en' => [
                    'title' => 'Summer tournament',
                    'slug' => 'summer-tournament',
                    'summary' => 'A deterministic tournament schedule.',
                    'body' => 'Summer tournament body',
                ],
            ],
        ];
    }

    private function createEvent(
        string $slug,
        string $status,
        CarbonImmutable $startsAt,
        CarbonImmutable $endsAt,
        string $title,
    ): Event {
        $event = Event::factory()->create([
            'status' => $status,
            'starts_at' => $startsAt,
            'ends_at' => $endsAt,
        ]);
        EventTranslation::factory()->create([
            'event_id' => $event->id,
            'locale' => 'en',
            'slug' => $slug,
            'title' => $title,
            'summary' => $title.' summary',
            'body' => $title.' body',
        ]);

        return $event;
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
            'key' => 'event-role-'.$identity->id,
            'name' => 'Event test role',
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

    /**
     * @param  list<string>  $permissions
     */
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

    private function freezeAt(CarbonImmutable $at): void
    {
        Carbon::setTestNow($at);
        CarbonImmutable::setTestNow($at);
    }
}
