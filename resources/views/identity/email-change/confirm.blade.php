@extends('identity.layout')

@section('title', __('identity.email_change.confirm.title'))

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">{{ __('identity.email_change.confirm.eyebrow') }}</p>
        <h1>{{ __('identity.email_change.confirm.heading') }}</h1>
        <p>{{ __('identity.email_change.confirm.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'identity.email-change.confirm.create',
            'localeParameters' => ['token' => $token],
        ])
    </div>

    <form method="POST" action="{{ route('identity.email-change.confirm', ['token' => $token]) }}" class="stacked-form">
        @csrf
        <button type="submit">{{ __('identity.email_change.confirm.submit') }}</button>
    </form>
@endsection
