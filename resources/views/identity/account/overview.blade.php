@extends('identity.layout')

@section('title', 'Account center')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')

    <header class="page-header">
        <p class="eyebrow">Account center</p>
        <h1>Account overview</h1>
        <p class="muted">Your Oteryn account brings together account readiness, security and the characters owned by your authenticated Platform identity.</p>
    </header>

    @php
        $badgeClass = match ($overview['state']) {
            'ready' => 'badge-success',
            'pending', 'recoverable' => 'badge-warning',
            default => 'badge-danger',
        };
    @endphp

    <div class="card-grid">
        <section class="card" aria-labelledby="game-account-heading">
            <p class="eyebrow">Game account</p>
            <h2 id="game-account-heading">Connection status</h2>
            <p><span class="badge {{ $badgeClass }}">{{ $overview['label'] }}</span></p>
            <p>{{ $overview['message'] }}</p>

            <div class="action-row">
                @if ($overview['character_creation_allowed'])
                    <a class="button" href="{{ route('account.characters.create') }}">Create a character</a>
                @endif

                @if ($overview['retry_allowed'])
                    <form method="POST" action="{{ route('account.provisioning.retry') }}">
                        @csrf
                        <button type="submit">Retry game account setup</button>
                    </form>
                @endif
            </div>
        </section>

        <section class="card" aria-labelledby="account-security-heading">
            <p class="eyebrow">Platform identity</p>
            <h2 id="account-security-heading">Security and access</h2>
            <dl>
                <dt>Email</dt>
                <dd>{{ $identity->email }}</dd>
                <dt>MFA</dt>
                <dd>
                    <span class="badge {{ $identity->hasConfirmedMfa() ? 'badge-success' : 'badge-warning' }}">
                        {{ $identity->hasConfirmedMfa() ? 'Enabled' : 'Not enabled' }}
                    </span>
                </dd>
            </dl>
            <div class="action-row">
                <a class="button button-secondary" href="{{ route('identity.mfa.settings') }}">Manage security</a>
                <a class="button button-secondary" href="{{ route('identity.password.change.create') }}">Change password</a>
            </div>
        </section>
    </div>

    <section class="panel" aria-labelledby="account-characters-heading">
        <div class="page-header">
            <p class="eyebrow">Game characters</p>
            <h2 id="account-characters-heading">Your characters</h2>
            <p class="muted">{{ $overview['characters_message'] }}</p>
        </div>

        @if ($overview['characters_state'] === 'available')
            <div class="card-grid">
                @foreach ($overview['characters'] as $character)
                    <article class="card">
                        <h3>
                            <a href="{{ route('game.characters.show', ['name' => $character->name]) }}">
                                {{ $character->name }}
                            </a>
                        </h3>
                        <dl>
                            <dt>Level</dt>
                            <dd>{{ $localeFormatter->number($character->level) }}</dd>
                            <dt>Vocation</dt>
                            <dd>{{ $characterPresentation->vocationName((int) $character->vocation) }}</dd>
                        </dl>
                        <div class="action-row">
                            <a class="button button-secondary" href="{{ route('game.characters.show', ['name' => $character->name]) }}">
                                View public profile
                            </a>
                        </div>
                    </article>
                @endforeach
            </div>
        @elseif ($overview['characters_state'] === 'empty')
            <div class="empty-state">
                <p>No active characters are attached to this game account.</p>
                @if ($overview['character_creation_allowed'])
                    <a class="button" href="{{ route('account.characters.create') }}">Create your first character</a>
                @endif
            </div>
        @elseif ($overview['characters_state'] === 'unavailable')
            <div class="notice alert-warning" role="status">
                The game account is ready, but character data cannot be read safely right now. Try again later.
            </div>
        @else
            <div class="notice" role="status">
                Character management becomes available after the game account connection reaches the Ready state.
            </div>
        @endif

        @if ($overview['state'] === 'ready' && ! $overview['character_creation_allowed'] && $overview['character_count'] >= $overview['character_limit'])
            <div class="notice alert-warning" role="status">
                This account has reached the limit of {{ $overview['character_limit'] }} active characters.
            </div>
        @endif
    </section>
@endsection
