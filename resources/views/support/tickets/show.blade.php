@extends('identity.layout')

@section('title', __('support.title.ticket_detail'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', [
        'localeRoute' => 'support.tickets.show',
        'localeParameters' => ['supportTicket' => $ticket],
    ])

    <p class="eyebrow">{{ $ticket->public_id }}</p>
    <h1>{{ $ticket->subject }}</h1>

    <dl class="detail-grid">
        <div><dt>{{ __('support.common.category') }}</dt><dd>{{ __('support.category.'.$ticket->category) }}</dd></div>
        <div><dt>{{ __('support.common.status') }}</dt><dd>{{ __('support.ticket_status.'.$ticket->status) }}</dd></div>
        <div><dt>{{ __('support.common.updated') }}</dt><dd>{{ $ticket->last_message_at->toDateTimeString() }}</dd></div>
    </dl>

    <h2>{{ __('support.tickets.conversation') }}</h2>
    <div class="message-list">
        @foreach ($messages as $message)
            <article class="card">
                <header>
                    <strong>{{ $message->author_kind === 'staff' ? __('support.nav.support_center') : __('support.tickets.owner') }}</strong>
                    <time datetime="{{ $message->created_at->toIso8601String() }}">{{ $message->created_at->toDateTimeString() }}</time>
                </header>
                <p class="pre-wrap">{{ $message->body }}</p>
            </article>
        @endforeach
    </div>

    @if ($ticket->allowsUserReply())
        <form method="POST" action="{{ route('support.tickets.reply', ['supportTicket' => $ticket, 'locale' => app()->getLocale()]) }}" class="form-stack">
            @csrf
            <input type="hidden" name="lock_version" value="{{ $ticket->lock_version }}">
            <label for="body">{{ __('support.tickets.reply') }}</label>
            <textarea id="body" name="body" rows="7" maxlength="8000" required placeholder="{{ __('support.tickets.reply_placeholder') }}">{{ old('body') }}</textarea>
            <button class="button" type="submit">{{ __('support.tickets.reply') }}</button>
        </form>
    @endif

    <form method="POST" action="{{ route('support.tickets.status', ['supportTicket' => $ticket, 'locale' => app()->getLocale()]) }}" class="action-row">
        @csrf
        @method('PUT')
        <input type="hidden" name="lock_version" value="{{ $ticket->lock_version }}">
        @if ($ticket->status === 'closed')
            <input type="hidden" name="status" value="open">
            <button class="button button-secondary" type="submit">{{ __('support.tickets.reopen') }}</button>
        @else
            <input type="hidden" name="status" value="closed">
            <button class="button button-secondary" type="submit">{{ __('support.tickets.close') }}</button>
        @endif
        <a class="button button-secondary" href="{{ route('support.tickets.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
    </form>
@endsection
