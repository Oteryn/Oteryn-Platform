@extends('identity.layout')

@section('title', __('identity.login.title'))
@section('error-title', __('identity.login.error_title'))

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('identity.login.eyebrow') }}</p>
        <h1>{{ __('identity.login.heading') }}</h1>
        <p class="muted">{{ __('identity.login.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'identity.login.create',
            'localeParameters' => [],
        ])
    </div>

    <form class="form-stack" method="POST" action="{{ route('identity.login.store', ['locale' => app()->getLocale()]) }}">
        @csrf
        <div class="form-field">
            <label for="email">{{ __('identity.login.email') }}</label>
            <input id="email" name="email" type="email" value="{{ old('email') }}" autocomplete="email" maxlength="254" required autofocus @error('email') aria-invalid="true" aria-describedby="error-email" @enderror>
        </div>
        <div class="form-field">
            <label for="password">{{ __('identity.login.password') }}</label>
            <input id="password" name="password" type="password" autocomplete="current-password" maxlength="1024" required @error('password') aria-invalid="true" aria-describedby="error-password" @enderror>
        </div>
        <button type="submit">{{ __('identity.login.submit') }}</button>
    </form>

    <nav class="identity-links" aria-label="{{ __('identity.login.help') }}">
        <a href="{{ route('password.request', ['locale' => app()->getLocale()]) }}">{{ __('identity.login.forgot_password') }}</a>
        <a href="{{ route('identity.register.create', ['locale' => app()->getLocale()]) }}">{{ __('identity.login.create_account') }}</a>
        <a href="{{ route('home') }}">{{ __('identity.login.return_public') }}</a>
    </nav>
@endsection
