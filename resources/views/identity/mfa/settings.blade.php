@extends('identity.layout')

@section('title', __('portal.identity.mfa_title'))
@section('error-title', __('portal.identity.mfa_error'))

@section('portal-family', 'security')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.identity.security') }}</p>
        <h1>{{ __('portal.identity.mfa_heading') }}</h1>
        <p class="muted">{{ __('portal.identity.mfa_intro') }}</p>
    </div>

    @if ($identity->hasConfirmedMfa())
        <div class="alert alert-success" role="status">
            <strong>{{ __('portal.identity.mfa_enabled') }}</strong> {{ __('portal.identity.mfa_signin') }}
        </div>
        <p class="muted">{{ __('portal.identity.mfa_disable_note') }}</p>

        <form class="form-stack" method="POST" action="{{ route('identity.mfa.destroy', ['locale' => app()->getLocale()]) }}">
            @csrf
            @method('DELETE')
            <div class="form-field">
                <label for="current_password">{{ __('portal.identity.current_password') }}</label>
                <input id="current_password" name="current_password" type="password" autocomplete="current-password" maxlength="1024" required @error('current_password') aria-invalid="true" aria-describedby="error-current-password" @enderror>
            </div>
            <div class="form-field">
                <label for="code">{{ __('portal.identity.mfa_fresh') }}</label>
                <input id="code" name="code" type="text" autocomplete="one-time-code" maxlength="64" required @error('code') aria-invalid="true" aria-describedby="error-code" @enderror>
            </div>
            <button class="danger" type="submit">{{ __('portal.identity.mfa_disable') }}</button>
        </form>
    @elseif (is_string($identity->two_factor_secret))
        <div class="alert alert-warning">
            <strong>{{ __('portal.identity.mfa_pending') }}</strong> {{ __('portal.identity.mfa_scan_confirm') }}
        </div>

        @if (is_string($qrCodeDataUri))
            <section class="mfa-qr-panel" aria-labelledby="mfa-qr-heading">
                <h2 id="mfa-qr-heading">{{ __('portal.identity.mfa_scan') }}</h2>
                <p class="muted">{{ __('portal.identity.mfa_google') }} <strong>{{ __('portal.identity.mfa_qr_action') }}</strong>.</p>
                <img
                    class="mfa-qr-code"
                    src="{{ $qrCodeDataUri }}"
                    width="280"
                    height="280"
                    alt="{{ __('portal.identity.mfa_qr_alt') }}"
                >
            </section>
        @endif

        <details class="secure-information mfa-manual-setup">
            <summary>{{ __('portal.identity.mfa_cannot_scan') }}</summary>
            <div>
                <p class="muted">{{ __('portal.identity.mfa_manual_help') }}</p>
                <p><strong>{{ __('portal.identity.mfa_secret') }}</strong> <code>{{ $identity->two_factor_secret }}</code></p>
            </div>
        </details>

        <form class="form-stack" method="POST" action="{{ route('identity.mfa.confirm', ['locale' => app()->getLocale()]) }}">
            @csrf
            <div class="form-field">
                <label for="current_password">{{ __('portal.identity.current_password') }}</label>
                <input id="current_password" name="current_password" type="password" autocomplete="current-password" maxlength="1024" required @error('current_password') aria-invalid="true" aria-describedby="error-current-password" @enderror>
            </div>
            <div class="form-field">
                <label for="code">{{ __('portal.identity.mfa_six') }}</label>
                <input id="code" name="code" type="text" inputmode="numeric" autocomplete="one-time-code" pattern="[0-9]{6}" maxlength="6" required @error('code') aria-invalid="true" aria-describedby="error-code" @enderror>
            </div>
            <button type="submit">{{ __('portal.identity.mfa_enable') }}</button>
        </form>
    @else
        <div class="panel">
            <h2>{{ __('portal.identity.mfa_disabled') }}</h2>
            <p class="muted">{{ __('portal.identity.mfa_enroll_help') }}</p>
            <form method="POST" action="{{ route('identity.mfa.enroll', ['locale' => app()->getLocale()]) }}">
                @csrf
                <button type="submit">{{ __('portal.identity.mfa_start') }}</button>
            </form>
        </div>
    @endif
@endsection
