<?php

namespace App\Support\Http;

use App\Identity\Models\Identity;
use App\Support\Actions\ManageSupportTicket;
use App\Support\Http\Requests\SupportTicketCreateRequest;
use App\Support\Http\Requests\SupportTicketReplyRequest;
use App\Support\Http\Requests\SupportTicketStatusRequest;
use App\Support\Models\SupportTicket;
use App\Support\Models\SupportTicketMessage;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Str;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class SupportTicketController
{
    public function index(Request $request): View
    {
        $identity = $this->identity($request);

        return view('support.tickets.index', [
            'tickets' => SupportTicket::query()
                ->where('identity_id', $identity->id)
                ->orderByDesc('last_message_at')
                ->paginate(20),
        ]);
    }

    public function create(Request $request): View
    {
        $this->identity($request);

        return view('support.tickets.create', [
            'categories' => SupportTicket::categories(),
            'requestKey' => (string) Str::uuid(),
            'attachmentsEnabled' => (bool) config('support.attachments.enabled', false),
        ]);
    }

    public function store(SupportTicketCreateRequest $request, ManageSupportTicket $tickets): RedirectResponse
    {
        $identity = $this->identity($request);

        try {
            $ticket = $tickets->create(
                $identity,
                $request->string('request_key')->toString(),
                $request->string('category')->toString(),
                $request->string('subject')->toString(),
                $request->string('body')->toString(),
            );
        } catch (DomainException $exception) {
            return back()->withInput()->withErrors(['ticket' => $exception->getMessage()]);
        }

        return redirect()->route('support.tickets.show', $ticket)->with('status', __('support.status.ticket_created'));
    }

    public function show(Request $request, SupportTicket $supportTicket): View
    {
        $identity = $this->identity($request);
        $this->assertOwner($identity, $supportTicket);

        return view('support.tickets.show', [
            'ticket' => $supportTicket,
            'messages' => SupportTicketMessage::query()
                ->where('support_ticket_id', $supportTicket->id)
                ->where('visibility', SupportTicketMessage::VISIBILITY_PUBLIC)
                ->orderBy('id')
                ->get(),
        ]);
    }

    public function reply(
        SupportTicketReplyRequest $request,
        SupportTicket $supportTicket,
        ManageSupportTicket $tickets,
    ): RedirectResponse {
        $identity = $this->identity($request);

        try {
            $tickets->userReply(
                $identity,
                $supportTicket,
                $request->string('body')->toString(),
                $request->integer('lock_version'),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('support.tickets.show', $supportTicket)->with('status', __('support.status.reply_added'));
    }

    public function status(
        SupportTicketStatusRequest $request,
        SupportTicket $supportTicket,
        ManageSupportTicket $tickets,
    ): RedirectResponse {
        $identity = $this->identity($request);
        $requested = $request->string('status')->toString();
        if (! in_array($requested, [SupportTicket::STATUS_CLOSED, SupportTicket::STATUS_OPEN], true)) {
            abort(422);
        }

        try {
            $tickets->userStatus(
                $identity,
                $supportTicket,
                $requested,
                $request->integer('lock_version'),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('support.tickets.show', $supportTicket)->with('status', __('support.status.ticket_status_changed'));
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }

    private function assertOwner(Identity $identity, SupportTicket $ticket): void
    {
        abort_unless($ticket->identity_id === $identity->id, 404);
    }
}
