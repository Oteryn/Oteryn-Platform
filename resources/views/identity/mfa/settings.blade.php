@extends('identity.layout')

@section('title', 'MFA settings')
@section('error-title', 'MFA settings could not be updated.')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Account security</p>
        <h1>Multi-factor authentication</h1>
        <p class="muted">Protect your Oteryn Platform web sign in with an authenticator app and single-use recovery codes.</p>
    </div>

    @if ($identity->hasConfirmedMfa())
        <div class="alert alert-success" role="status">
            <strong>MFA is enabled.</strong> Future Oteryn Platform web sign ins require a second factor.
        </div>
        <p class="muted">Disabling MFA signs out every Platform web session.</p>

        <form class="form-stack" method="POST" action="{{ route('identity.mfa.destroy') }}">
            @csrf
            @method('DELETE')
            <div class="form-field">
                <label for="current_password">Current password</label>
                <input id="current_password" name="current_password" type="password" autocomplete="current-password" maxlength="1024" required>
            </div>
            <div class="form-field">
                <label for="code">Fresh authenticator or recovery code</label>
                <input id="code" name="code" type="text" autocomplete="one-time-code" maxlength="64" required>
            </div>
            <button class="danger" type="submit">Disable MFA and sign out everywhere</button>
        </form>
    @elseif (is_string($identity->two_factor_secret))
        <div class="alert alert-warning">
            <strong>Enrollment is not active yet.</strong> Add the account to your authenticator app, then confirm a fresh six-digit code below.
        </div>

        <div class="secure-information" aria-labelledby="manual-mfa-heading">
            <h2 id="manual-mfa-heading">Manual authenticator setup</h2>
            <p class="muted">Use the manual secret when your authenticator app supports key entry. The provisioning URI is provided for apps that accept URI import.</p>
            <p><strong>Manual secret:</strong> <code>{{ $identity->two_factor_secret }}</code></p>
            <p><strong>Provisioning URI:</strong> <code>{{ $provisioningUri }}</code></p>
        </div>

        <form class="form-stack" method="POST" action="{{ route('identity.mfa.confirm') }}">
            @csrf
            <div class="form-field">
                <label for="current_password">Current password</label>
                <input id="current_password" name="current_password" type="password" autocomplete="current-password" maxlength="1024" required>
            </div>
            <div class="form-field">
                <label for="code">Six-digit authenticator code</label>
                <input id="code" name="code" type="text" inputmode="numeric" autocomplete="one-time-code" pattern="[0-9]{6}" maxlength="6" required>
            </div>
            <button type="submit">Confirm and enable MFA</button>
        </form>
    @else
        <div class="panel">
            <h2>MFA is not enabled</h2>
            <p class="muted">Enabling MFA will require a second factor for future Oteryn Platform web sign ins.</p>
            <form method="POST" action="{{ route('identity.mfa.enroll') }}">
                @csrf
                <button type="submit">Start MFA enrollment</button>
            </form>
        </div>
    @endif
@endsection
