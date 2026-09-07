@extends('identity.layout')

@section('title', __('portal.identity.register_title'))
@section('error-title', __('portal.identity.register_error'))

@section('portal-family', 'identity')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.identity.new_account') }}</p>
        <h1>{{ __('portal.identity.create_identity') }}</h1>
        <p class="muted">{{ __('portal.identity.register_intro') }}</p>
    </div>

    <form class="form-stack" method="POST" action="{{ route('identity.register.store', ['locale' => app()->getLocale()]) }}">
        @csrf
        <div class="form-field">
            <label for="email">{{ __('portal.identity.email') }}</label>
            <input id="email" name="email" type="email" value="{{ old('email') }}" autocomplete="email" maxlength="254" required autofocus @error('email') aria-invalid="true" aria-describedby="error-email" @enderror>
        </div>
        <div class="form-field">
            <label for="password">{{ __('portal.identity.password') }}</label>
            <input id="password" name="password" type="password" autocomplete="new-password" maxlength="1024" required @error('password') aria-invalid="true" aria-describedby="error-password" @enderror>
        </div>
        <div class="form-field">
            <label for="password_confirmation">{{ __('portal.identity.confirm_password') }}</label>
            <input id="password_confirmation" name="password_confirmation" type="password" autocomplete="new-password" maxlength="1024" required @error('password_confirmation') aria-invalid="true" aria-describedby="error-password-confirmation" @enderror>
        </div>
        <button type="submit">{{ __('portal.identity.register') }}</button>
    </form>

    <nav class="identity-links" aria-label="{{ __('portal.identity.register_nav') }}">
        <a href="{{ route('identity.login.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.identity.already_registered') }}</a>
        <a href="{{ route('home') }}">{{ __('portal.identity.return_public') }}</a>
    </nav>
@endsection
