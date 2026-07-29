@extends('identity.layout')

@section('title', __('support.title.ticket_create'))

@section('content')
    @include('support.partials.navigation')
    @include('identity.partials.locale-switcher', ['localeRoute' => 'support.tickets.create'])

    <p class="eyebrow">{{ __('support.nav.support_center') }}</p>
    <h1>{{ __('support.title.ticket_create') }}</h1>
    <p class="supporting-copy">{{ __('support.common.attachments_disabled') }}</p>

    <form method="POST" action="{{ route('support.tickets.store', ['locale' => app()->getLocale()]) }}" class="form-stack">
        @csrf
        <input type="hidden" name="request_key" value="{{ old('request_key', $requestKey) }}">

        <label for="category">{{ __('support.common.category') }}</label>
        <select id="category" name="category" required>
            @foreach ($categories as $category)
                <option value="{{ $category }}" @selected(old('category') === $category)>{{ __('support.category.'.$category) }}</option>
            @endforeach
        </select>

        <label for="subject">{{ __('support.common.subject') }}</label>
        <input id="subject" name="subject" type="text" maxlength="160" required value="{{ old('subject') }}">

        <label for="body">{{ __('support.tickets.initial_message') }}</label>
        <textarea id="body" name="body" rows="10" maxlength="8000" required>{{ old('body') }}</textarea>

        <div class="action-row">
            <button class="button" type="submit">{{ __('support.tickets.open_new') }}</button>
            <a class="button button-secondary" href="{{ route('support.tickets.index', ['locale' => app()->getLocale()]) }}">{{ __('support.common.back') }}</a>
        </div>
    </form>
@endsection
