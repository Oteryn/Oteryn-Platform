<?php

namespace App\Support\Http;

use App\Identity\Models\Identity;
use App\Support\Actions\ManageSupportTicket;
use App\Support\Http\Requests\SupportTicketReplyRequest;
use App\Support\Http\Requests\SupportTicketStatusRequest;
use App\Support\Models\SupportTicket;
use App\Support\Models\SupportTicketMessage;
use DomainException;
use Illuminate\Contracts\View\View;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\ConflictHttpException;

final class AdminSupportTicketController
{
    public function index(Request $request): View
    {
        $status = $request->query('status');
        $query = SupportTicket::query()->with('identity')->orderByDesc('last_message_at');
        if (is_string($status) && in_array($status, SupportTicket::statuses(), true)) {
            $query->where('status', $status);
        }

        return view('admin.support.tickets.index', [
            'tickets' => $query->paginate(30)->withQueryString(),
            'statuses' => SupportTicket::statuses(),
            'selectedStatus' => is_string($status) ? $status : null,
        ]);
    }

    public function show(SupportTicket $supportTicket): View
    {
        return view('admin.support.tickets.show', [
            'ticket' => $supportTicket->load('identity'),
            'messages' => SupportTicketMessage::query()
                ->where('support_ticket_id', $supportTicket->id)
                ->orderBy('id')
                ->get(),
            'statuses' => SupportTicket::statuses(),
        ]);
    }

    public function reply(
        SupportTicketReplyRequest $request,
        SupportTicket $supportTicket,
        ManageSupportTicket $tickets,
    ): RedirectResponse {
        $actor = $this->identity($request);

        try {
            $tickets->staffReply(
                $actor,
                $supportTicket,
                $request->string('body')->toString(),
                $request->boolean('internal'),
                $request->integer('lock_version'),
                app()->getLocale(),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('admin.support.tickets.show', $supportTicket)->with('status', __('support.status.reply_added'));
    }

    public function status(
        SupportTicketStatusRequest $request,
        SupportTicket $supportTicket,
        ManageSupportTicket $tickets,
    ): RedirectResponse {
        $actor = $this->identity($request);

        try {
            $tickets->staffStatus(
                $actor,
                $supportTicket,
                $request->string('status')->toString(),
                $request->integer('lock_version'),
                app()->getLocale(),
            );
        } catch (DomainException $exception) {
            throw new ConflictHttpException($exception->getMessage(), $exception);
        }

        return redirect()->route('admin.support.tickets.show', $supportTicket)->with('status', __('support.status.ticket_status_changed'));
    }

    private function identity(Request $request): Identity
    {
        $identity = $request->user();
        abort_unless($identity instanceof Identity, 403);

        return $identity;
    }
}
