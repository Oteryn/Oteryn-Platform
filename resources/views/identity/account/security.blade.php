@extends('identity.layout')

@section('title', __('identity.security.title'))

@section('content')
    <header class="page-header">
        <p class="eyebrow">{{ __('identity.security.eyebrow') }}</p>
        <h1>{{ __('identity.security.heading') }}</h1>
        <p class="muted">{{ __('identity.security.intro') }}</p>
        @include('identity.partials.locale-switcher', [
            'localeRoute' => 'identity.account-security.show',
            'localeParameters' => [],
        ])
    </header>

    <div class="card-grid">
        <section class="card" aria-labelledby="primary-email-heading">
            <p class="eyebrow">{{ __('identity.security.email.eyebrow') }}</p>
            <h2 id="primary-email-heading">{{ __('identity.security.email.heading') }}</h2>
            <p>{{ __('identity.security.email.current') }} <strong>{{ $identity->email }}</strong></p>

            @if ($pendingEmailChange)
                <div class="notice alert-warning" role="status">
                    {{ __('identity.security.email.pending_prefix') }} <strong>{{ $pendingEmailChange->new_email }}</strong>
                    {{ __('identity.common.until') }}
                    <time datetime="{{ $pendingEmailChange->expires_at->toAtomString() }}">{{ $pendingEmailChange->expires_at->utc()->format('Y-m-d H:i') }} UTC</time>.
                    {{ __('identity.security.email.pending_recovery') }}
                </div>
            @endif

            @if ($identity->email_change_available_at?->isFuture())
                <p class="muted">{{ __('identity.security.email.cooldown', ['date' => $identity->email_change_available_at->utc()->format('Y-m-d H:i')]) }}</p>
            @else
                <form method="POST" action="{{ route('identity.email-change.store') }}" class="stacked-form">
                    @csrf
                    <label for="new-email">
                        <span>{{ __('identity.security.email.new') }}</span>
                        <input id="new-email" type="email" name="email" maxlength="254" autocomplete="email" required>
                    </label>
                    <label for="new-email-confirmation">
                        <span>{{ __('identity.security.email.confirm_new') }}</span>
                        <input id="new-email-confirmation" type="email" name="email_confirmation" maxlength="254" autocomplete="email" required>
                    </label>
                    <label for="email-current-password">
                        <span>{{ __('identity.common.current_password') }}</span>
                        <input id="email-current-password" type="password" name="current_password" autocomplete="current-password" required>
                    </label>
                    <button type="submit">{{ __('identity.security.email.submit') }}</button>
                </form>
            @endif
        </section>

        <section class="card" aria-labelledby="privacy-heading">
            <p class="eyebrow">{{ __('identity.security.privacy.eyebrow') }}</p>
            <h2 id="privacy-heading">{{ __('identity.security.privacy.heading') }}</h2>
            <p class="muted">{{ __('identity.security.privacy.intro') }}</p>
            <form method="POST" action="{{ route('identity.privacy.update') }}" class="stacked-form">
                @csrf
                @method('PUT')
                <label class="checkbox-row">
                    <input type="checkbox" name="public_account_association" value="1" @checked($identity->public_account_association)>
                    <span>{{ __('identity.security.privacy.association') }}</span>
                </label>
                <label class="checkbox-row">
                    <input type="checkbox" name="public_status_visible" value="1" @checked($identity->public_status_visible)>
                    <span>{{ __('identity.security.privacy.status') }}</span>
                </label>
                <button type="submit">{{ __('identity.security.privacy.submit') }}</button>
            </form>
        </section>
    </div>

    <section class="panel" aria-labelledby="sessions-heading">
        <div class="page-header">
            <p class="eyebrow">{{ __('identity.security.sessions.eyebrow') }}</p>
            <h2 id="sessions-heading">{{ __('identity.security.sessions.heading') }}</h2>
            <p class="muted">{{ __('identity.security.sessions.intro') }}</p>
        </div>

        @if ($sessions->isEmpty())
            <div class="empty-state"><p>{{ __('identity.security.sessions.empty') }}</p></div>
        @else
            <div class="card-grid">
                @foreach ($sessions as $webSession)
                    <article class="card">
                        <h3>
                            {{ $webSession->id === $currentSessionId ? __('identity.security.sessions.current_session') : __('identity.security.sessions.other_session') }}
                            @if ($webSession->id === $currentSessionId)
                                <span class="badge badge-success">{{ __('identity.security.sessions.current_badge') }}</span>
                            @endif
                        </h3>
                        <dl>
                            <dt>{{ __('identity.security.sessions.browser') }}</dt>
                            <dd>{{ $webSession->user_agent ?: __('identity.security.sessions.unknown_browser') }}</dd>
                            <dt>{{ __('identity.security.sessions.signed_in') }}</dt>
                            <dd><time datetime="{{ $webSession->issued_at->toAtomString() }}">{{ $webSession->issued_at->utc()->format('Y-m-d H:i') }} UTC</time></dd>
                            <dt>{{ __('identity.security.sessions.last_active') }}</dt>
                            <dd><time datetime="{{ $webSession->last_seen_at->toAtomString() }}">{{ $webSession->last_seen_at->utc()->format('Y-m-d H:i') }} UTC</time></dd>
                        </dl>
                        <form method="POST" action="{{ route('identity.sessions.destroy', ['session' => $webSession->id]) }}">
                            @csrf
                            @method('DELETE')
                            <button class="button button-danger" type="submit">
                                {{ $webSession->id === $currentSessionId ? __('identity.security.sessions.revoke_current') : __('identity.security.sessions.revoke') }}
                            </button>
                        </form>
                    </article>
                @endforeach
            </div>

            @if ($sessions->where('id', '!=', $currentSessionId)->isNotEmpty())
                <form method="POST" action="{{ route('identity.sessions.destroy-others') }}" class="action-row">
                    @csrf
                    @method('DELETE')
                    <button class="button button-danger" type="submit">{{ __('identity.security.sessions.revoke_others') }}</button>
                </form>
            @endif
        @endif
    </section>

    <div class="card-grid">
        <section class="card" aria-labelledby="recovery-key-heading">
            <p class="eyebrow">{{ __('identity.security.recovery.eyebrow') }}</p>
            <h2 id="recovery-key-heading">{{ __('identity.security.recovery.heading') }}</h2>
            <p>{{ $hasRecoveryKey ? __('identity.security.recovery.active') : __('identity.security.recovery.inactive') }}</p>
            <p class="muted">{{ __('identity.security.recovery.intro') }}</p>

            <form method="POST" action="{{ route('identity.recovery-key.generate') }}" class="stacked-form">
                @csrf
                <label for="recovery-key-password">
                    <span>{{ __('identity.common.current_password') }}</span>
                    <input id="recovery-key-password" type="password" name="current_password" autocomplete="current-password" required>
                </label>
                <button type="submit">{{ $hasRecoveryKey ? __('identity.security.recovery.rotate') : __('identity.security.recovery.generate') }}</button>
            </form>

            @if ($hasRecoveryKey)
                <form method="POST" action="{{ route('identity.recovery-key.revoke') }}" class="stacked-form">
                    @csrf
                    @method('DELETE')
                    <label for="recovery-key-revoke-password">
                        <span>{{ __('identity.common.current_password') }}</span>
                        <input id="recovery-key-revoke-password" type="password" name="current_password" autocomplete="current-password" required>
                    </label>
                    <button class="button button-danger" type="submit">{{ __('identity.security.recovery.revoke') }}</button>
                </form>
            @endif
        </section>

        <section class="card" aria-labelledby="factor-policy-heading">
            <p class="eyebrow">{{ __('identity.security.policy.eyebrow') }}</p>
            <h2 id="factor-policy-heading">{{ __('identity.security.policy.heading') }}</h2>
            <dl>
                <dt>{{ __('identity.security.policy.authenticator') }}</dt>
                <dd><span class="badge {{ $identity->hasConfirmedMfa() ? 'badge-success' : 'badge-warning' }}">{{ $identity->hasConfirmedMfa() ? __('identity.common.enabled') : __('identity.common.not_enabled') }}</span></dd>
                <dt>{{ __('identity.security.policy.email_code') }}</dt>
                <dd>{{ $emailCodeMfaEnabled ? __('identity.common.enabled') : __('identity.security.policy.email_code_not_adopted') }}</dd>
                <dt>{{ __('identity.security.policy.binding') }}</dt>
                <dd>{{ $bindingMutationPolicy === 'deny' ? __('identity.security.policy.binding_locked') : __('identity.security.policy.binding_managed') }}</dd>
            </dl>
            <div class="action-row">
                <a class="button button-secondary" href="{{ route('identity.mfa.settings') }}">{{ __('identity.security.policy.manage_authenticator') }}</a>
                <a class="button button-secondary" href="{{ route('identity.password.change.create') }}">{{ __('identity.security.policy.change_password') }}</a>
            </div>
        </section>
    </div>

    <section class="panel" aria-labelledby="termination-heading">
        <div class="page-header">
            <p class="eyebrow">{{ __('identity.security.termination.eyebrow') }}</p>
            <h2 id="termination-heading">{{ __('identity.security.termination.heading') }}</h2>
            <p class="muted">{{ __('identity.security.termination.intro') }}</p>
        </div>

        @if ($identity->hasPendingTermination())
            <div class="notice alert-warning" role="status">
                {{ __('identity.security.termination.scheduled_prefix') }}
                <time datetime="{{ $identity->termination_scheduled_for->toAtomString() }}">{{ $identity->termination_scheduled_for->utc()->format('Y-m-d H:i') }} UTC</time>.
            </div>
            <form method="POST" action="{{ route('identity.termination.destroy') }}" class="stacked-form">
                @csrf
                @method('DELETE')
                <label for="termination-cancel-password">
                    <span>{{ __('identity.common.current_password') }}</span>
                    <input id="termination-cancel-password" type="password" name="current_password" autocomplete="current-password" required>
                </label>
                <button type="submit">{{ __('identity.security.termination.cancel') }}</button>
            </form>
        @else
            <form method="POST" action="{{ route('identity.termination.store') }}" class="stacked-form">
                @csrf
                <label for="termination-password">
                    <span>{{ __('identity.common.current_password') }}</span>
                    <input id="termination-password" type="password" name="current_password" autocomplete="current-password" required>
                </label>
                <label for="termination-confirmation">
                    <span>{{ __('identity.security.termination.confirmation', ['phrase' => config('identity_security.termination.confirmation_phrase', 'TERMINATE')]) }}</span>
                    <input id="termination-confirmation" type="text" name="confirmation" autocomplete="off" required>
                </label>
                <button class="button button-danger" type="submit">{{ __('identity.security.termination.schedule') }}</button>
            </form>
        @endif
    </section>
@endsection
