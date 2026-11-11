@extends('admin.layout')

@section('title', __('support.title.admin_tickets'))

@section('content')
    <div class="section-heading">
        <div><p class="eyebrow">{{ __('support.nav.admin_tickets') }}</p><h1>{{ __('support.title.admin_tickets') }}</h1></div>
    </div>

    <form method="GET" action="{{ route('admin.support.tickets.index') }}" class="filter-row">
        <input type="hidden" name="locale" value="{{ app()->getLocale() }}">
        <label for="status">{{ __('support.common.status') }}</label>
        <select id="status" name="status">
            <option value="">{{ __('support.common.all_statuses') }}</option>
            @foreach ($statuses as $status)
                <option value="{{ $status }}" @selected($selectedStatus === $status)>{{ __('support.ticket_status.'.$status) }}</option>
            @endforeach
        </select>
        <button class="button button-secondary" type="submit">{{ __('support.common.filter') }}</button>
    </form>

    @if ($tickets->isEmpty())
        <div class="empty-state"><p>{{ __('support.tickets.queue_empty') }}</p></div>
    @else
        <div class="table-wrap">
            <table>
                <thead><tr><th>{{ __('support.common.subject') }}</th><th>{{ __('support.tickets.owner') }}</th><th>{{ __('support.common.status') }}</th><th>{{ __('support.common.updated') }}</th><th></th></tr></thead>
                <tbody>
                @foreach ($tickets as $ticket)
                    <tr>
                        <td>{{ $ticket->subject }}</td>
                        <td>#{{ $ticket->identity_id }}</td>
                        <td>{{ __('support.ticket_status.'.$ticket->status) }}</td>
                        <td>{{ $ticket->last_message_at->toDateTimeString() }}</td>
                        <td><a href="{{ route('admin.support.tickets.show', ['supportTicket' => $ticket, 'locale' => app()->getLocale()]) }}">{{ __('support.common.view') }}</a></td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
        {{ $tickets->links() }}
    @endif
@endsection
