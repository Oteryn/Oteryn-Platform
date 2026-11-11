@extends('identity.layout')

@section('title', __('portal.identity.challenge_title'))
@section('error-title', __('portal.identity.challenge_error'))

@section('portal-family', 'identity')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.identity.second_factor') }}</p>
        <h1>{{ __('portal.identity.challenge_heading') }}</h1>
        <p class="muted">{{ __('portal.identity.challenge_intro') }}</p>
    </div>

    <form class="form-stack" method="POST" action="{{ route('identity.mfa.challenge.store', ['locale' => app()->getLocale()]) }}">
        @csrf
        <div class="form-field">
            <label for="code">{{ __('portal.identity.challenge_code') }}</label>
            <input id="code" name="code" type="text" autocomplete="one-time-code" maxlength="64" required autofocus @error('code') aria-invalid="true" aria-describedby="error-code" @enderror>
        </div>
        <button type="submit">{{ __('portal.identity.challenge_verify') }}</button>
    </form>

    <nav class="identity-links" aria-label="{{ __('portal.identity.challenge_nav') }}">
        <a href="{{ route('identity.login.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.identity.challenge_restart') }}</a>
        <a href="{{ route('home') }}">{{ __('portal.identity.return_public') }}</a>
    </nav>
@endsection
