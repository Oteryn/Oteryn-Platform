@extends('identity.layout')

@section('title', __('identity.email_change.recover.title'))

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">{{ __('identity.email_change.recover.eyebrow') }}</p>
        <h1>{{ __('identity.email_change.recover.heading') }}</h1>
        <p>{{ __('identity.email_change.recover.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'identity.email-change.recover.create',
            'localeParameters' => ['token' => $token],
        ])
    </div>

    <form method="POST" action="{{ route('identity.email-change.recover', ['token' => $token]) }}" class="stacked-form">
        @csrf
        <button class="button button-danger" type="submit">{{ __('identity.email_change.recover.submit') }}</button>
    </form>
@endsection
