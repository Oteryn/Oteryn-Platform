@extends('identity.layout')

@section('title', 'Recover account')

@section('content')
    <div class="identity-heading">
        <p class="eyebrow">High-assurance recovery</p>
        <h1>Recover your account with a recovery key</h1>
        <p>A valid single-use recovery key resets the password, removes MFA and revokes every existing web session and game authorization.</p>
    </div>

    <form method="POST" action="{{ route('identity.recovery-key.recover') }}" class="stacked-form">
        @csrf
        <label for="recovery-email">
            <span>Email address</span>
            <input id="recovery-email" type="email" name="email" value="{{ old('email') }}" maxlength="254" autocomplete="email" required autofocus>
        </label>
        <label for="recovery-key">
            <span>Recovery key</span>
            <input id="recovery-key" type="text" name="recovery_key" maxlength="160" autocomplete="off" required>
        </label>
        <label for="recovery-password">
            <span>New password</span>
            <input id="recovery-password" type="password" name="password" autocomplete="new-password" required>
        </label>
        <label for="recovery-password-confirmation">
            <span>Confirm new password</span>
            <input id="recovery-password-confirmation" type="password" name="password_confirmation" autocomplete="new-password" required>
        </label>
        <button type="submit">Recover account</button>
    </form>

    <p class="muted"><a href="{{ route('password.request') }}">Use email password recovery instead</a></p>
@endsection
