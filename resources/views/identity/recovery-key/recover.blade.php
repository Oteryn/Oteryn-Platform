@extends('identity.layout')

@section('title', __('identity.recovery_key.recover.title'))

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">{{ __('identity.recovery_key.recover.eyebrow') }}</p>
        <h1>{{ __('identity.recovery_key.recover.heading') }}</h1>
        <p>{{ __('identity.recovery_key.recover.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'identity.recovery-key.recover.create',
            'localeParameters' => [],
        ])
    </div>

    <form method="POST" action="{{ route('identity.recovery-key.recover') }}" class="stacked-form">
        @csrf
        <label for="recovery-email">
            <span>{{ __('identity.recovery_key.recover.email') }}</span>
            <input id="recovery-email" type="email" name="email" value="{{ old('email') }}" maxlength="254" autocomplete="email" required autofocus>
        </label>
        <label for="recovery-key">
            <span>{{ __('identity.recovery_key.recover.key') }}</span>
            <input id="recovery-key" type="text" name="recovery_key" maxlength="160" autocomplete="off" required>
        </label>
        <label for="recovery-password">
            <span>{{ __('identity.recovery_key.recover.new_password') }}</span>
            <input id="recovery-password" type="password" name="password" autocomplete="new-password" required>
        </label>
        <label for="recovery-password-confirmation">
            <span>{{ __('identity.recovery_key.recover.confirm_password') }}</span>
            <input id="recovery-password-confirmation" type="password" name="password_confirmation" autocomplete="new-password" required>
        </label>
        <button type="submit">{{ __('identity.recovery_key.recover.submit') }}</button>
    </form>

    <p class="muted"><a href="{{ route('password.request') }}">{{ __('identity.recovery_key.recover.email_alternative') }}</a></p>
@endsection
