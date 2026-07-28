<?php

namespace App\Support\Actions;

use App\Audit\AdminAuditRecorder;
use App\Identity\Models\Identity;
use App\Support\Models\SupportTicket;
use App\Support\Models\SupportTicketMessage;
use App\Support\Notifications\SupportNotificationDeliveryService;
use DomainException;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Str;
use InvalidArgumentException;

final class ManageSupportTicket
{
    public function __construct(
        private readonly AdminAuditRecorder $audit,
        private readonly SupportNotificationDeliveryService $notifications,
    ) {}

    public function create(
        Identity $identity,
        string $requestKey,
        string $category,
        string $subject,
        string $body,
    ): SupportTicket {
        $this->assertCategory($category);
        $normalizedSubject = trim($subject);
        $normalizedBody = trim($body);

        if ($normalizedSubject === '' || $normalizedBody === '') {
            throw new InvalidArgumentException('A subject and initial message are required.');
        }

        return DB::transaction(function () use ($identity, $requestKey, $category, $normalizedSubject, $normalizedBody): SupportTicket {
            $lockedIdentity = Identity::query()->lockForUpdate()->findOrFail($identity->id);
            abort_if($lockedIdentity->isTerminated(), 403);

            $existing = SupportTicket::query()
                ->where('identity_id', $lockedIdentity->id)
                ->where('request_key', $requestKey)
                ->first();
            if ($existing instanceof SupportTicket) {
                return $existing;
            }

            $openCount = SupportTicket::query()
                ->where('identity_id', $lockedIdentity->id)
                ->whereNotIn('status', [SupportTicket::STATUS_RESOLVED, SupportTicket::STATUS_CLOSED])
                ->count();
            if ($openCount >= (int) config('support.tickets.open_limit_per_identity', 10)) {
                throw new DomainException('The open support-ticket limit has been reached.');
            }

            $ticket = SupportTicket::query()->create([
                'public_id' => (string) Str::ulid(),
                'identity_id' => $lockedIdentity->id,
                'request_key' => $requestKey,
                'category' => $category,
                'subject' => $normalizedSubject,
                'status' => SupportTicket::STATUS_OPEN,
                'lock_version' => 1,
                'last_message_at' => now(),
            ]);

            SupportTicketMessage::query()->create([
                'support_ticket_id' => $ticket->id,
                'author_identity_id' => $lockedIdentity->id,
                'author_kind' => SupportTicketMessage::AUTHOR_USER,
                'visibility' => SupportTicketMessage::VISIBILITY_PUBLIC,
                'body' => $normalizedBody,
            ]);

            return $ticket;
        }, 3);
    }

    public function userReply(Identity $identity, SupportTicket $ticket, string $body, int $expectedLockVersion): SupportTicket
    {
        $normalizedBody = trim($body);
        if ($normalizedBody === '') {
            throw new InvalidArgumentException('A reply is required.');
        }

        return DB::transaction(function () use ($identity, $ticket, $normalizedBody, $expectedLockVersion): SupportTicket {
            $current = SupportTicket::query()->lockForUpdate()->findOrFail($ticket->id);
            $this->assertOwner($identity, $current);
            $this->assertVersion($current, $expectedLockVersion);

            if (! $current->allowsUserReply()) {
                throw new DomainException('This ticket does not currently accept user replies.');
            }

            SupportTicketMessage::query()->create([
                'support_ticket_id' => $current->id,
                'author_identity_id' => $identity->id,
                'author_kind' => SupportTicketMessage::AUTHOR_USER,
                'visibility' => SupportTicketMessage::VISIBILITY_PUBLIC,
                'body' => $normalizedBody,
            ]);

            $current->forceFill([
                'status' => SupportTicket::STATUS_WAITING_STAFF,
                'lock_version' => $current->lock_version + 1,
                'last_message_at' => now(),
                'closed_at' => null,
            ])->save();

            return $current;
        }, 3);
    }

    public function userStatus(
        Identity $identity,
        SupportTicket $ticket,
        string $status,
        int $expectedLockVersion,
    ): SupportTicket {
        if (! in_array($status, [SupportTicket::STATUS_CLOSED, SupportTicket::STATUS_OPEN], true)) {
            throw new InvalidArgumentException('Unsupported user ticket status.');
        }

        return DB::transaction(function () use ($identity, $ticket, $status, $expectedLockVersion): SupportTicket {
            $current = SupportTicket::query()->lockForUpdate()->findOrFail($ticket->id);
            $this->assertOwner($identity, $current);
            $this->assertVersion($current, $expectedLockVersion);

            if ($status === SupportTicket::STATUS_OPEN && $current->status !== SupportTicket::STATUS_CLOSED) {
                throw new DomainException('Only a closed ticket can be reopened.');
            }

            $current->forceFill([
                'status' => $status,
                'lock_version' => $current->lock_version + 1,
                'closed_at' => $status === SupportTicket::STATUS_CLOSED ? now() : null,
            ])->save();

            return $current;
        }, 3);
    }

    public function staffReply(
        Identity $actor,
        SupportTicket $ticket,
        string $body,
        bool $internal,
        int $expectedLockVersion,
        string $locale = 'en',
    ): SupportTicket {
        $normalizedBody = trim($body);
        if ($normalizedBody === '') {
            throw new InvalidArgumentException('A reply or internal note is required.');
        }

        $saved = DB::transaction(function () use ($actor, $ticket, $normalizedBody, $internal, $expectedLockVersion): SupportTicket {
            $current = SupportTicket::query()->lockForUpdate()->findOrFail($ticket->id);
            $this->assertVersion($current, $expectedLockVersion);

            if (in_array($current->status, [SupportTicket::STATUS_RESOLVED, SupportTicket::STATUS_CLOSED], true)) {
                throw new DomainException('Resolved or closed tickets cannot receive staff replies.');
            }

            SupportTicketMessage::query()->create([
                'support_ticket_id' => $current->id,
                'author_identity_id' => $actor->id,
                'author_kind' => SupportTicketMessage::AUTHOR_STAFF,
                'visibility' => $internal
                    ? SupportTicketMessage::VISIBILITY_INTERNAL
                    : SupportTicketMessage::VISIBILITY_PUBLIC,
                'body' => $normalizedBody,
            ]);

            $current->forceFill([
                'status' => $internal ? $current->status : SupportTicket::STATUS_WAITING_USER,
                'lock_version' => $current->lock_version + 1,
                'last_message_at' => now(),
            ])->save();

            $this->audit->record(
                $actor->id,
                $internal ? 'support.ticket_internal_note_added' : 'support.ticket_replied',
                'support_ticket',
                $current->public_id,
                [
                    'status' => $current->status,
                    'internal' => $internal,
                    'lock_version' => $current->lock_version,
                ],
            );

            return $current;
        }, 3);

        if (! $internal) {
            $owner = Identity::query()->find($saved->identity_id);
            if ($owner instanceof Identity) {
                $this->notifications->queue(
                    $owner,
                    SupportNotificationDeliveryService::TICKET_REPLY,
                    'ticket',
                    $saved->public_id,
                    $locale,
                );
            }
        }

        return $saved;
    }

    public function staffStatus(
        Identity $actor,
        SupportTicket $ticket,
        string $status,
        int $expectedLockVersion,
        string $locale = 'en',
    ): SupportTicket {
        if (! in_array($status, SupportTicket::statuses(), true)) {
            throw new InvalidArgumentException('Unsupported ticket status.');
        }

        $saved = DB::transaction(function () use ($actor, $ticket, $status, $expectedLockVersion): SupportTicket {
            $current = SupportTicket::query()->lockForUpdate()->findOrFail($ticket->id);
            $this->assertVersion($current, $expectedLockVersion);

            $current->forceFill([
                'status' => $status,
                'lock_version' => $current->lock_version + 1,
                'closed_at' => in_array($status, [SupportTicket::STATUS_RESOLVED, SupportTicket::STATUS_CLOSED], true)
                    ? now()
                    : null,
            ])->save();

            $this->audit->record(
                $actor->id,
                'support.ticket_status_changed',
                'support_ticket',
                $current->public_id,
                ['status' => $status, 'lock_version' => $current->lock_version],
            );

            return $current;
        }, 3);

        $owner = Identity::query()->find($saved->identity_id);
        if ($owner instanceof Identity) {
            $this->notifications->queue(
                $owner,
                SupportNotificationDeliveryService::TICKET_STATUS,
                'ticket',
                $saved->public_id,
                $locale,
            );
        }

        return $saved;
    }

    private function assertCategory(string $category): void
    {
        if (! in_array($category, SupportTicket::categories(), true)) {
            throw new InvalidArgumentException('Unsupported support-ticket category.');
        }
    }

    private function assertOwner(Identity $identity, SupportTicket $ticket): void
    {
        if ($ticket->identity_id !== $identity->id) {
            abort(404);
        }
    }

    private function assertVersion(SupportTicket $ticket, int $expectedLockVersion): void
    {
        if ($ticket->lock_version !== $expectedLockVersion) {
            throw new DomainException('This ticket changed after the page was opened. Reload it before continuing.');
        }
    }
}
