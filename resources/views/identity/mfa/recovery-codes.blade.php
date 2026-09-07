@extends('identity.layout')

@section('title', __('portal.identity.codes_title'))

@section('portal-family', 'security')

@section('content')
    <div class="page-header">
        <p class="eyebrow">{{ __('portal.identity.security') }}</p>
        <h1>{{ __('portal.identity.codes_heading') }}</h1>
        <p class="muted">{{ __('portal.identity.codes_intro') }}</p>
    </div>

    <div class="alert alert-warning">
        {{ __('portal.identity.codes_warning') }}
    </div>

    <ul class="recovery-codes" aria-label="{{ __('portal.identity.codes_title') }}">
        @foreach ($recoveryCodes as $recoveryCode)
            <li><code>{{ $recoveryCode }}</code></li>
        @endforeach
    </ul>

    <div class="action-row">
        <a class="button" href="{{ route('identity.mfa.settings', ['locale' => app()->getLocale()]) }}">{{ __('portal.identity.mfa_return') }}</a>
        <a class="button button-secondary" href="{{ route('home') }}">{{ __('portal.identity.go_public') }}</a>
    </div>
@endsection
