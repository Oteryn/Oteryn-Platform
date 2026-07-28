@extends('identity.layout')

@section('title', __('identity.recovery_key.generated.title'))

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">{{ __('identity.recovery_key.generated.eyebrow') }}</p>
        <h1>{{ __('identity.recovery_key.generated.heading') }}</h1>
        <p>{{ __('identity.recovery_key.generated.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'identity.account-security.show',
            'localeParameters' => [],
        ])
    </div>

    <div class="recovery-codes" role="status" aria-label="{{ __('identity.recovery_key.generated.label') }}">
        <code>{{ $recoveryKey }}</code>
    </div>

    <p class="muted">{{ __('identity.recovery_key.generated.verifier') }}</p>
    <div class="action-row">
        <a class="button" href="{{ route('identity.account-security.show') }}">{{ __('identity.recovery_key.generated.return') }}</a>
    </div>
@endsection
