@extends('identity.layout')

@section('title', __('portal.identity.forgot_title'))
@section('error-title', __('portal.identity.forgot_error'))

@section('portal-family', 'identity')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.identity.recovery') }}</p>
        <h1>{{ __('portal.identity.forgot_heading') }}</h1>
        <p class="muted">{{ __('portal.identity.forgot_intro') }}</p>
    </div>

    <form class="form-stack" method="POST" action="{{ route('password.email', ['locale' => app()->getLocale()]) }}">
        @csrf
        <div class="form-field">
            <label for="email">{{ __('portal.identity.email') }}</label>
            <input id="email" name="email" type="email" value="{{ old('email') }}" autocomplete="email" maxlength="254" required autofocus @error('email') aria-invalid="true" aria-describedby="error-email" @enderror>
        </div>
        <button type="submit">{{ __('portal.identity.send_reset') }}</button>
    </form>

    <nav class="identity-links" aria-label="{{ __('portal.identity.recovery_nav') }}">
        <a href="{{ route('identity.login.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.identity.return_login') }}</a>
        <a href="{{ route('identity.register.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.identity.create_account') }}</a>
    </nav>
@endsection
