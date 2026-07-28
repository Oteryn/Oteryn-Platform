@extends('identity.layout')

@section('title', 'Account security')

@section('content')
    <header class="page-header">
        <p class="eyebrow">Account center</p>
        <h1>Security and lifecycle</h1>
        <p class="muted">Manage your primary email, active sessions, privacy, recovery key and termination grace period. Sensitive changes are rate-limited and audited.</p>
    </header>

    <div class="card-grid">
        <section class="card" aria-labelledby="primary-email-heading">
            <p class="eyebrow">Primary email</p>
            <h2 id="primary-email-heading">Change email address</h2>
            <p>Current address: <strong>{{ $identity->email }}</strong></p>

            @if ($pendingEmailChange)
                <div class="notice alert-warning" role="status">
                    Confirmation is pending for <strong>{{ $pendingEmailChange->new_email }}</strong> until
                    <time datetime="{{ $pendingEmailChange->expires_at->toAtomString() }}">{{ $pendingEmailChange->expires_at->utc()->format('Y-m-d H:i') }} UTC</time>.
                    The previous-address recovery link can cancel the request.
                </div>
            @endif

            @if ($identity->email_change_available_at?->isFuture())
                <p class="muted">Another change becomes available after {{ $identity->email_change_available_at->utc()->format('Y-m-d H:i') }} UTC.</p>
            @else
                <form method="POST" action="{{ route('identity.email-change.store') }}" class="stacked-form">
                    @csrf
                    <label for="new-email">
                        <span>New email address</span>
                        <input id="new-email" type="email" name="email" maxlength="254" autocomplete="email" required>
                    </label>
                    <label for="new-email-confirmation">
                        <span>Confirm new email address</span>
                        <input id="new-email-confirmation" type="email" name="email_confirmation" maxlength="254" autocomplete="email" required>
                    </label>
                    <label for="email-current-password">
                        <span>Current password</span>
                        <input id="email-current-password" type="password" name="current_password" autocomplete="current-password" required>
                    </label>
                    <button type="submit">Send confirmation links</button>
                </form>
            @endif
        </section>

        <section class="card" aria-labelledby="privacy-heading">
            <p class="eyebrow">Privacy</p>
            <h2 id="privacy-heading">Public account signals</h2>
            <p class="muted">These settings default to private. Future character-profile features must read them server-side.</p>
            <form method="POST" action="{{ route('identity.privacy.update') }}" class="stacked-form">
                @csrf
                @method('PUT')
                <label class="checkbox-row">
                    <input type="checkbox" name="public_account_association" value="1" @checked($identity->public_account_association)>
                    <span>Allow public characters to show an account association when that feature is delivered.</span>
                </label>
                <label class="checkbox-row">
                    <input type="checkbox" name="public_status_visible" value="1" @checked($identity->public_status_visible)>
                    <span>Allow public online/status visibility when supported by a public profile.</span>
                </label>
                <button type="submit">Save privacy settings</button>
            </form>
        </section>
    </div>

    <section class="panel" aria-labelledby="sessions-heading">
        <div class="page-header">
            <p class="eyebrow">Active sessions</p>
            <h2 id="sessions-heading">Signed-in browsers</h2>
            <p class="muted">Oteryn stores a bounded browser summary and a protected source-address fingerprint. Raw source addresses are not displayed.</p>
        </div>

        @if ($sessions->isEmpty())
            <div class="empty-state"><p>No active registered sessions were found.</p></div>
        @else
            <div class="card-grid">
                @foreach ($sessions as $webSession)
                    <article class="card">
                        <h3>
                            {{ $webSession->id === $currentSessionId ? 'Current session' : 'Other session' }}
                            @if ($webSession->id === $currentSessionId)
                                <span class="badge badge-success">Current</span>
                            @endif
                        </h3>
                        <dl>
                            <dt>Browser</dt>
                            <dd>{{ $webSession->user_agent ?: 'Unknown browser' }}</dd>
                            <dt>Signed in</dt>
                            <dd><time datetime="{{ $webSession->issued_at->toAtomString() }}">{{ $webSession->issued_at->utc()->format('Y-m-d H:i') }} UTC</time></dd>
                            <dt>Last active</dt>
                            <dd><time datetime="{{ $webSession->last_seen_at->toAtomString() }}">{{ $webSession->last_seen_at->utc()->format('Y-m-d H:i') }} UTC</time></dd>
                        </dl>
                        <form method="POST" action="{{ route('identity.sessions.destroy', ['session' => $webSession->id]) }}">
                            @csrf
                            @method('DELETE')
                            <button class="button button-danger" type="submit">
                                {{ $webSession->id === $currentSessionId ? 'Revoke and sign out' : 'Revoke session' }}
                            </button>
                        </form>
                    </article>
                @endforeach
            </div>

            @if ($sessions->where('id', '!=', $currentSessionId)->isNotEmpty())
                <form method="POST" action="{{ route('identity.sessions.destroy-others') }}" class="action-row">
                    @csrf
                    @method('DELETE')
                    <button class="button button-danger" type="submit">Revoke all other sessions</button>
                </form>
            @endif
        @endif
    </section>

    <div class="card-grid">
        <section class="card" aria-labelledby="recovery-key-heading">
            <p class="eyebrow">High-assurance recovery</p>
            <h2 id="recovery-key-heading">Recovery key</h2>
            <p>{{ $hasRecoveryKey ? 'An active recovery key exists.' : 'No active recovery key exists.' }}</p>
            <p class="muted">The raw key is displayed once. Store it offline. Generating a new key invalidates the previous key.</p>

            <form method="POST" action="{{ route('identity.recovery-key.generate') }}" class="stacked-form">
                @csrf
                <label for="recovery-key-password">
                    <span>Current password</span>
                    <input id="recovery-key-password" type="password" name="current_password" autocomplete="current-password" required>
                </label>
                <button type="submit">{{ $hasRecoveryKey ? 'Rotate recovery key' : 'Generate recovery key' }}</button>
            </form>

            @if ($hasRecoveryKey)
                <form method="POST" action="{{ route('identity.recovery-key.revoke') }}" class="stacked-form">
                    @csrf
                    @method('DELETE')
                    <label for="recovery-key-revoke-password">
                        <span>Current password</span>
                        <input id="recovery-key-revoke-password" type="password" name="current_password" autocomplete="current-password" required>
                    </label>
                    <button class="button button-danger" type="submit">Revoke recovery key</button>
                </form>
            @endif
        </section>

        <section class="card" aria-labelledby="factor-policy-heading">
            <p class="eyebrow">Authentication policy</p>
            <h2 id="factor-policy-heading">MFA and game-account binding</h2>
            <dl>
                <dt>Authenticator app</dt>
                <dd><span class="badge {{ $identity->hasConfirmedMfa() ? 'badge-success' : 'badge-warning' }}">{{ $identity->hasConfirmedMfa() ? 'Enabled' : 'Not enabled' }}</span></dd>
                <dt>Email-code MFA</dt>
                <dd>{{ $emailCodeMfaEnabled ? 'Enabled' : 'Not adopted — email remains the recovery channel.' }}</dd>
                <dt>Game-account binding</dt>
                <dd>{{ $bindingMutationPolicy === 'deny' ? 'Locked by policy. Self-service unlink or rebind is not allowed.' : 'Managed under the configured exceptional policy.' }}</dd>
            </dl>
            <div class="action-row">
                <a class="button button-secondary" href="{{ route('identity.mfa.settings') }}">Manage authenticator app</a>
                <a class="button button-secondary" href="{{ route('identity.password.change.create') }}">Change password</a>
            </div>
        </section>
    </div>

    <section class="panel" aria-labelledby="termination-heading">
        <div class="page-header">
            <p class="eyebrow">Account termination</p>
            <h2 id="termination-heading">Platform access grace period</h2>
            <p class="muted">Termination disables and anonymizes Platform login after the grace period. It does not delete or transfer the bound Canary account or characters.</p>
        </div>

        @if ($identity->hasPendingTermination())
            <div class="notice alert-warning" role="status">
                Termination is scheduled for
                <time datetime="{{ $identity->termination_scheduled_for->toAtomString() }}">{{ $identity->termination_scheduled_for->utc()->format('Y-m-d H:i') }} UTC</time>.
            </div>
            <form method="POST" action="{{ route('identity.termination.destroy') }}" class="stacked-form">
                @csrf
                @method('DELETE')
                <label for="termination-cancel-password">
                    <span>Current password</span>
                    <input id="termination-cancel-password" type="password" name="current_password" autocomplete="current-password" required>
                </label>
                <button type="submit">Cancel termination</button>
            </form>
        @else
            <form method="POST" action="{{ route('identity.termination.store') }}" class="stacked-form">
                @csrf
                <label for="termination-password">
                    <span>Current password</span>
                    <input id="termination-password" type="password" name="current_password" autocomplete="current-password" required>
                </label>
                <label for="termination-confirmation">
                    <span>Type {{ config('identity_security.termination.confirmation_phrase', 'TERMINATE') }} to confirm</span>
                    <input id="termination-confirmation" type="text" name="confirmation" autocomplete="off" required>
                </label>
                <button class="button button-danger" type="submit">Schedule account termination</button>
            </form>
        @endif
    </section>
@endsection
