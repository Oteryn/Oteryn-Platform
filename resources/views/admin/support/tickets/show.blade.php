@extends('admin.layout')

@section('title', __('support.title.ticket_detail'))

@section('content')
    <p class="eyebrow">{{ $ticket->public_id }}</p>
    <h1>{{ $ticket->subject }}</h1>

    <dl class="detail-grid">
        <div><dt>{{ __('support.tickets.owner') }}</dt><dd>#{{ $ticket->identity_id }} · {{ $ticket->identity?->email }}</dd></div>
        <div><dt>{{ __('support.common.category') }}</dt><dd>{{ __('support.category.'.$ticket->category) }}</dd></div>
        <div><dt>{{ __('support.common.status') }}</dt><dd>{{ __('support.ticket_status.'.$ticket->status) }}</dd></div>
        <div><dt>{{ __('support.common.updated') }}</dt><dd>{{ $ticket->last_message_at->toDateTimeString() }}</dd></div>
    </dl>

    <h2>{{ __('support.tickets.conversation') }}</h2>
    <div class="message-list">
        @foreach ($messages as $message)
            <article class="card">
                <header>
                    <strong>{{ $message->author_kind }}</strong>
                    @if ($message->visibility === 'internal')<span class="badge">{{ __('support.tickets.internal_badge') }}</span>@endif
                    <time datetime="{{ $message->created_at->toIso8601String() }}">{{ $message->created_at->toDateTimeString() }}</time>
                </header>
                <p class="pre-wrap">{{ $message->body }}</p>
            </article>
        @endforeach
    </div>

    @if (! in_array($ticket->status, ['resolved', 'closed'], true))
        <form method="POST" action="{{ route('admin.support.tickets.reply', ['supportTicket' => $ticket, 'locale' => app()->getLocale()]) }}" class="form-stack">
            @csrf
            <input type="hidden" name="lock_version" value="{{ $ticket->lock_version }}">
            <label for="body">{{ __('support.tickets.reply') }}</label>
            <textarea id="body" name="body" rows="8" maxlength="8000" required>{{ old('body') }}</textarea>
            <label><input type="checkbox" name="internal" value="1" @checked(old('internal'))> {{ __('support.tickets.internal_note') }}</label>
            <button class="button" type="submit">{{ __('support.tickets.reply') }}</button>
        </form>
    @endif

    <form method="POST" action="{{ route('admin.support.tickets.status', ['supportTicket' => $ticket, 'locale' => app()->getLocale()]) }}" class="form-stack">
        @csrf
        @method('PUT')
        <input type="hidden" name="lock_version" value="{{ $ticket->lock_version }}">
        <label for="status">{{ __('support.common.status') }}</label>
        <select id="status" name="status" required>
            @foreach ($statuses as $status)
                <option value="{{ $status }}" @selected($ticket->status === $status)>{{ __('support.ticket_status.'.$status) }}</option>
            @endforeach
        </select>
        <button class="button button-secondary" type="submit">{{ __('support.common.save') }}</button>
    </form>

    <a class="button button-secondary" href="{{ route('admin.support.tickets.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
@endsection
