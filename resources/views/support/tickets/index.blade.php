@extends('identity.layout')

@section('title', __('support.title.tickets'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', ['localeRoute' => 'support.tickets.index'])

    <div class="section-heading">
        <div>
            <p class="eyebrow">{{ __('support.nav.support_center') }}</p>
            <h1>{{ __('support.title.tickets') }}</h1>
        </div>
        <a class="button" href="{{ route('support.tickets.create', ['locale' => app()->getLocale()]) }}">{{ __('support.tickets.open_new') }}</a>
    </div>

    @if ($tickets->isEmpty())
        <div class="empty-state"><p>{{ __('support.tickets.empty') }}</p></div>
    @else
        <div class="table-wrap">
            <table>
                <thead>
                <tr>
                    <th>{{ __('support.common.subject') }}</th>
                    <th>{{ __('support.common.category') }}</th>
                    <th>{{ __('support.common.status') }}</th>
                    <th>{{ __('support.common.updated') }}</th>
                    <th><span class="sr-only">{{ __('support.common.view') }}</span></th>
                </tr>
                </thead>
                <tbody>
                @foreach ($tickets as $ticket)
                    <tr>
                        <td>{{ $ticket->subject }}</td>
                        <td>{{ __('support.category.'.$ticket->category) }}</td>
                        <td>{{ __('support.ticket_status.'.$ticket->status) }}</td>
                        <td>{{ $ticket->last_message_at->toDateTimeString() }}</td>
                        <td><a href="{{ route('support.tickets.show', ['supportTicket' => $ticket, 'locale' => app()->getLocale()]) }}">{{ __('support.common.view') }}</a></td>
                    </tr>
                @endforeach
                </tbody>
            </table>
        </div>
        {{ $tickets->links() }}
    @endif
@endsection
