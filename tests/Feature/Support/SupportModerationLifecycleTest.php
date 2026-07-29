<?php

namespace Tests\Feature\Support;

use App\Identity\Models\Identity;
use App\Support\Actions\ManageSupportTicket;
use App\Support\Models\EnforcementRecord;
use App\Support\Models\PlayerReport;
use App\Support\Models\SupportNotificationDelivery;
use App\Support\Models\SupportTicket;
use App\Support\Models\SupportTicketMessage;
use App\Support\Notifications\SupportNotificationDeliveryService;
use App\Support\Retention\PruneSupportRetention;
use Carbon\CarbonImmutable;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Notification;
use Illuminate\Support\Str;
use RuntimeException;
use Tests\TestCase;

final class SupportModerationLifecycleTest extends TestCase
{
    use RefreshDatabase;

    public function test_guest_redirects_preserve_locale_and_user_routes_require_authentication(): void
    {
        $this->get(route('support.tickets.index', ['locale' => 'pl']))
            ->assertRedirect(route('identity.login.create', ['locale' => 'pl']));

        $this->get(route('support.reports.index'))->assertRedirect();
        $this->get(route('support.enforcement.index'))->assertRedirect();
    }

    public function test_ticket_lifecycle_is_idempotent_owner_scoped_and_conflict_safe(): void
    {
        $owner = $this->createIdentity('ticket-owner@example.com');
        $other = $this->createIdentity('ticket-other@example.com');
        $requestKey = (string) Str::uuid();

        $this->actingAs($owner)->post(route('support.tickets.store'), [
            'request_key' => $requestKey,
            'category' => SupportTicket::CATEGORY_ACCOUNT,
            'subject' => 'Cannot access a character',
            'body' => 'The affected character is Example Knight.',
        ])->assertRedirect();

        $ticket = SupportTicket::query()->firstOrFail();
        self::assertSame(1, SupportTicket::query()->count());
        self::assertSame(1, SupportTicketMessage::query()->count());

        $this->actingAs($owner)->post(route('support.tickets.store'), [
            'request_key' => $requestKey,
            'category' => SupportTicket::CATEGORY_ACCOUNT,
            'subject' => 'A conflicting duplicate subject',
            'body' => 'A conflicting duplicate body.',
        ])->assertRedirect(route('support.tickets.show', $ticket));
        self::assertSame(1, SupportTicket::query()->count());
        self::assertSame(1, SupportTicketMessage::query()->count());

        $this->actingAs($other)->get(route('support.tickets.show', $ticket))->assertNotFound();

        $initialVersion = $ticket->lock_version;
        $this->actingAs($owner)->post(route('support.tickets.reply', $ticket), [
            'body' => 'Additional user evidence.',
            'lock_version' => $initialVersion,
        ])->assertRedirect();

        $ticket->refresh();
        self::assertSame(SupportTicket::STATUS_WAITING_STAFF, $ticket->status);
        self::assertSame($initialVersion + 1, $ticket->lock_version);

        $this->actingAs($owner)->post(route('support.tickets.reply', $ticket), [
            'body' => 'Stale duplicate reply.',
            'lock_version' => $initialVersion,
        ])->assertConflict();
        self::assertSame(2, SupportTicketMessage::query()->count());

        $this->actingAs($owner)->put(route('support.tickets.status', $ticket), [
            'status' => SupportTicket::STATUS_CLOSED,
            'lock_version' => $ticket->lock_version,
        ])->assertRedirect();
        $ticket->refresh();
        self::assertSame(SupportTicket::STATUS_CLOSED, $ticket->status);
    }

    public function test_ticket_moderation_requires_mfa_and_exact_permission_and_hides_internal_notes(): void
    {
        Notification::fake();
        $owner = $this->createIdentity('ticket-user@example.com');
        $ticket = app(ManageSupportTicket::class)->create(
            $owner,
            (string) Str::uuid(),
            SupportTicket::CATEGORY_TECHNICAL,
            'Client crash',
            'The client closes after login.',
        );

        $noMfa = $this->createIdentity('support-no-mfa@example.com', false);
        $this->grantPermissions($noMfa, ['support.tickets.manage']);
        $this->actingAs($noMfa)->get(route('admin.support.tickets.index'))->assertForbidden();

        $wrongPermission = $this->createIdentity('support-wrong-permission@example.com');
        $this->actingAs($wrongPermission)->get(route('admin.support.tickets.index'))->assertForbidden();

        $moderator = $this->createIdentity('support-moderator@example.com');
        $this->grantPermissions($moderator, ['support.tickets.manage']);

        $this->actingAs($moderator)->post(route('admin.support.tickets.reply', $ticket), [
            'body' => 'Please reinstall the signed current client.',
            'internal' => false,
            'lock_version' => $ticket->lock_version,
        ])->assertRedirect();

        $ticket->refresh();
        $this->actingAs($moderator)->post(route('admin.support.tickets.reply', $ticket), [
            'body' => 'PRIVATE-MODERATOR-DIAGNOSIS',
            'internal' => true,
            'lock_version' => $ticket->lock_version,
        ])->assertRedirect();

        $this->actingAs($owner)->get(route('support.tickets.show', $ticket))
            ->assertOk()
            ->assertSeeText('Please reinstall the signed current client.')
            ->assertDontSeeText('PRIVATE-MODERATOR-DIAGNOSIS');

        $audit = json_encode(DB::table('admin_audit_events')->get()->all(), JSON_THROW_ON_ERROR);
        self::assertStringNotContainsString('Please reinstall the signed current client.', $audit);
        self::assertStringNotContainsString('PRIVATE-MODERATOR-DIAGNOSIS', $audit);
        $this->assertDatabaseHas('admin_audit_events', [
            'action' => 'support.ticket_internal_note_added',
            'target_type' => 'support_ticket',
            'target_id' => $ticket->public_id,
        ]);
        $this->assertDatabaseHas('support_notification_deliveries', [
            'identity_id' => $owner->id,
            'event_key' => SupportNotificationDeliveryService::TICKET_REPLY,
            'related_id' => $ticket->public_id,
        ]);
    }

    public function test_reports_are_bounded_idempotent_private_and_moderated_with_public_safe_outcomes(): void
    {
        Notification::fake();
        config(['support.reports.pending_limit_per_identity' => 1]);
        $reporter = $this->createIdentity('reporter@example.com');
        $other = $this->createIdentity('other-reporter@example.com');
        $requestKey = (string) Str::uuid();

        $payload = [
            'request_key' => $requestKey,
            'report_type' => PlayerReport::TYPE_PLAYER,
            'category' => 'cheating',
            'target_reference' => 'Example Player',
            'evidence_summary' => 'PRIVATE-REPORT-EVIDENCE',
        ];
        $this->actingAs($reporter)->post(route('support.reports.store'), $payload)->assertRedirect();
        $report = PlayerReport::query()->firstOrFail();
        $this->actingAs($reporter)->post(route('support.reports.store'), $payload)->assertRedirect();
        self::assertSame(1, PlayerReport::query()->count());

        $this->actingAs($reporter)->from(route('support.reports.create'))->post(route('support.reports.store'), [
            ...$payload,
            'request_key' => (string) Str::uuid(),
            'report_type' => PlayerReport::TYPE_GUILD,
            'category' => 'cheating',
        ])->assertSessionHasErrors('category');

        $this->actingAs($reporter)->from(route('support.reports.create'))->post(route('support.reports.store'), [
            ...$payload,
            'request_key' => (string) Str::uuid(),
            'target_reference' => 'Second Player',
        ])->assertSessionHasErrors('report');
        self::assertSame(1, PlayerReport::query()->count());

        $this->actingAs($other)->get(route('support.reports.show', $report))->assertNotFound();

        $moderator = $this->createIdentity('report-moderator@example.com');
        $this->grantPermissions($moderator, ['support.reports.manage']);
        $version = $report->lock_version;
        $this->actingAs($moderator)->put(route('admin.moderation.reports.update', $report), [
            'status' => PlayerReport::STATUS_ACTIONED,
            'public_outcome' => 'The report was reviewed and appropriate action was taken.',
            'moderator_notes' => 'PRIVATE-MODERATOR-REPORT-NOTE',
            'lock_version' => $version,
        ])->assertRedirect();

        $this->actingAs($moderator)->put(route('admin.moderation.reports.update', $report), [
            'status' => PlayerReport::STATUS_CLOSED,
            'public_outcome' => 'Stale overwrite attempt.',
            'moderator_notes' => 'Stale private note.',
            'lock_version' => $version,
        ])->assertConflict();

        $this->actingAs($reporter)->get(route('support.reports.show', $report))
            ->assertOk()
            ->assertSeeText('The report was reviewed and appropriate action was taken.')
            ->assertDontSeeText('PRIVATE-MODERATOR-REPORT-NOTE');

        $audit = json_encode(DB::table('admin_audit_events')->get()->all(), JSON_THROW_ON_ERROR);
        self::assertStringNotContainsString('PRIVATE-REPORT-EVIDENCE', $audit);
        self::assertStringNotContainsString('PRIVATE-MODERATOR-REPORT-NOTE', $audit);
        $this->assertDatabaseHas('support_notification_deliveries', [
            'identity_id' => $reporter->id,
            'event_key' => SupportNotificationDeliveryService::REPORT_OUTCOME,
            'related_id' => $report->public_id,
        ]);
    }

    public function test_enforcement_history_is_owner_scoped_appealable_audited_and_platform_only(): void
    {
        Notification::fake();
        $target = $this->createIdentity('enforcement-target@example.com');
        $other = $this->createIdentity('enforcement-other@example.com');
        $moderator = $this->createIdentity('enforcement-moderator@example.com');
        $this->grantPermissions($moderator, ['support.enforcement.manage']);

        $this->actingAs($moderator)->post(route('admin.moderation.enforcement.store'), [
            'target_identity_id' => $target->id,
            'category' => EnforcementRecord::CATEGORY_WARNING,
            'status' => EnforcementRecord::STATUS_ACTIVE,
            'public_reason' => 'Repeated violation of the published conduct rule.',
            'moderator_notes' => 'PRIVATE-ENFORCEMENT-NOTE',
            'effective_at' => now()->utc()->format('Y-m-d\TH:i'),
            'expires_at' => now()->addDays(30)->utc()->format('Y-m-d\TH:i'),
        ])->assertRedirect();

        $record = EnforcementRecord::query()->firstOrFail();
        $this->actingAs($other)->get(route('support.enforcement.show', $record))->assertNotFound();
        $this->actingAs($target)->get(route('support.enforcement.show', $record))
            ->assertOk()
            ->assertSeeText('Repeated violation of the published conduct rule.')
            ->assertSeeText('Platform enforcement record')
            ->assertDontSeeText('PRIVATE-ENFORCEMENT-NOTE');

        $this->actingAs($target)->post(route('support.enforcement.acknowledge', $record), [
            'lock_version' => $record->lock_version,
        ])->assertRedirect();
        $record->refresh();

        $this->actingAs($target)->post(route('support.enforcement.appeal', $record), [
            'appeal_message' => 'Please review the evidence associated with this warning.',
            'lock_version' => $record->lock_version,
        ])->assertRedirect();
        $record->refresh();
        self::assertSame(EnforcementRecord::APPEAL_REQUESTED, $record->appeal_status);

        $version = $record->lock_version;
        $this->actingAs($moderator)->put(route('admin.moderation.enforcement.appeal', $record), [
            'appeal_status' => EnforcementRecord::APPEAL_ACCEPTED,
            'appeal_outcome' => 'The warning was revoked after review.',
            'lock_version' => $version,
        ])->assertRedirect();

        $this->actingAs($moderator)->put(route('admin.moderation.enforcement.appeal', $record), [
            'appeal_status' => EnforcementRecord::APPEAL_REJECTED,
            'appeal_outcome' => 'Stale decision.',
            'lock_version' => $version,
        ])->assertConflict();

        $this->actingAs($target)->get(route('support.enforcement.show', $record))
            ->assertOk()
            ->assertSeeText('The warning was revoked after review.')
            ->assertDontSeeText('PRIVATE-ENFORCEMENT-NOTE');

        $audit = json_encode(DB::table('admin_audit_events')->get()->all(), JSON_THROW_ON_ERROR);
        self::assertStringNotContainsString('PRIVATE-ENFORCEMENT-NOTE', $audit);
        self::assertStringNotContainsString('Please review the evidence associated with this warning.', $audit);
        $this->assertDatabaseHas('support_notification_deliveries', [
            'identity_id' => $target->id,
            'event_key' => SupportNotificationDeliveryService::APPEAL_OUTCOME,
            'related_id' => $record->public_id,
        ]);
    }

    public function test_notification_failure_does_not_rollback_ticket_status(): void
    {
        $owner = $this->createIdentity('terminated-notification@example.com');
        $moderator = $this->createIdentity('notification-moderator@example.com');
        $ticket = app(ManageSupportTicket::class)->create(
            $owner,
            (string) Str::uuid(),
            SupportTicket::CATEGORY_OTHER,
            'A notification failure test',
            'Initial body.',
        );
        $owner->forceFill(['terminated_at' => now()])->save();

        app(ManageSupportTicket::class)->staffStatus(
            $moderator,
            $ticket,
            SupportTicket::STATUS_RESOLVED,
            $ticket->lock_version,
        );

        $delivery = SupportNotificationDelivery::query()->firstOrFail();
        app(SupportNotificationDeliveryService::class)->deliver($delivery->id);

        $ticket->refresh();
        $delivery->refresh();
        self::assertSame(SupportTicket::STATUS_RESOLVED, $ticket->status);
        self::assertSame(SupportNotificationDelivery::STATUS_FAILED, $delivery->status);
        self::assertSame('notifiable_unavailable', $delivery->last_error_code);
    }

    public function test_retention_supports_dry_run_then_deletes_or_anonymizes_only_expired_records(): void
    {
        $identity = $this->createIdentity('retention@example.com');
        $old = CarbonImmutable::now()->subDays(1200);

        $ticket = SupportTicket::query()->create([
            'public_id' => (string) Str::ulid(),
            'identity_id' => $identity->id,
            'request_key' => (string) Str::uuid(),
            'category' => SupportTicket::CATEGORY_OTHER,
            'subject' => 'Old ticket',
            'status' => SupportTicket::STATUS_CLOSED,
            'lock_version' => 1,
            'last_message_at' => $old,
            'closed_at' => $old,
            'created_at' => $old,
            'updated_at' => $old,
        ]);
        SupportTicketMessage::query()->create([
            'support_ticket_id' => $ticket->id,
            'author_identity_id' => $identity->id,
            'author_kind' => SupportTicketMessage::AUTHOR_USER,
            'visibility' => SupportTicketMessage::VISIBILITY_PUBLIC,
            'body' => 'Old body',
        ]);
        $report = PlayerReport::query()->create([
            'public_id' => (string) Str::ulid(),
            'reporter_identity_id' => $identity->id,
            'request_key' => (string) Str::uuid(),
            'report_type' => PlayerReport::TYPE_CONTENT,
            'category' => 'spam',
            'target_reference' => 'old-content',
            'evidence_summary' => 'Old evidence',
            'status' => PlayerReport::STATUS_CLOSED,
            'public_outcome' => 'Closed',
            'moderator_notes' => 'Old private note',
            'lock_version' => 1,
            'processed_at' => $old,
            'created_at' => $old,
            'updated_at' => $old,
        ]);
        $record = EnforcementRecord::query()->create([
            'public_id' => (string) Str::ulid(),
            'identity_id' => $identity->id,
            'category' => EnforcementRecord::CATEGORY_WARNING,
            'status' => EnforcementRecord::STATUS_EXPIRED,
            'public_reason' => 'Retained public reason',
            'moderator_notes' => 'Old private enforcement note',
            'effective_at' => $old->subDay(),
            'expires_at' => $old,
            'appeal_status' => EnforcementRecord::APPEAL_REJECTED,
            'appeal_message' => 'Old appeal message',
            'lock_version' => 1,
            'created_at' => $old,
            'updated_at' => $old,
        ]);

        $pruner = app(PruneSupportRetention::class);
        self::assertSame([
            'tickets_deleted' => 1,
            'reports_deleted' => 1,
            'enforcement_anonymized' => 1,
        ], $pruner->execute(true));
        self::assertNotNull($ticket->fresh());
        self::assertNotNull($report->fresh());
        $record->refresh();
        self::assertSame('Old private enforcement note', $record->moderator_notes);

        $pruner->execute();
        self::assertNull($ticket->fresh());
        self::assertNull($report->fresh());
        $record->refresh();
        self::assertNull($record->moderator_notes);
        self::assertNull($record->appeal_message);
        self::assertSame('Retained public reason', $record->public_reason);
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
            'key' => 'support-role-'.$identity->id,
            'name' => 'Support test role',
            'created_at' => $now,
            'updated_at' => $now,
        ]);

        foreach ($permissions as $permission) {
            DB::table('admin_role_permissions')->insert([
                'role_id' => $roleId,
                'permission_id' => $this->integerDatabaseValue(
                    DB::table('admin_permissions')->where('key', $permission)->value('id'),
                    "permission {$permission}",
                ),
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
