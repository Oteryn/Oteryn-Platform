@extends('identity.layout')
@section('title', __('portal.account.title'))
@section('portal-family', 'account')
@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')
    <header class="page-header account-masthead">
        <div>
            <p class="eyebrow">{{ __('portal.account.workspace') }}</p>
            <h1>{{ __('portal.account.heading') }}</h1>
            <p class="muted">{{ __('portal.account.intro') }}</p>
        </div>
        @if ($overview['character_creation_allowed'])
            <a class="button" href="{{ route('account.characters.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.create') }} <span aria-hidden="true">＋</span></a>
        @endif
    </header>
    @if ($overview['native'] ?? false)
        @include('identity.account.partials.native-roster')
    @else
    @php
        $badgeClass = match ($overview['state']) {
            'ready' => 'badge-success',
            default => 'badge-danger',
        };
        $charactersMessage = $overview['characters_state'] === 'available'
            ? __($overview['character_count'] === 1
                ? '1 of :limit active character slots is in use.'
                : ':count of :limit active character slots are in use.', [
                    'count' => $localeFormatter->number($overview['character_count']),
                    'limit' => $localeFormatter->number($overview['character_limit']),
                ])
            : __($overview['characters_message']);
    @endphp
    @if ($overview['state'] === 'none')
    <span hidden data-account-state="none"></span>
    @else
    <section class="account-connection" aria-labelledby="game-account-heading" data-account-state="{{ $overview['state'] }}">
        <div>
            <p class="eyebrow">{{ __('portal.account.game_account') }}</p>
            <h2 id="game-account-heading">{{ __('portal.account.connection') }}</h2>
        </div>
        <div>
            <span class="badge {{ $badgeClass }}">{{ __($overview['label']) }}</span>
            <p>{{ __($overview['message']) }}</p>
        </div>
    </section>
    <section class="account-roster" aria-labelledby="account-characters-heading">
        <div class="section-heading">
            <div><p class="eyebrow">{{ __('portal.account.game_account') }}</p><h2 id="account-characters-heading">{{ __('portal.account.characters') }}</h2></div>
            @if (in_array($overview['characters_state'], ['available', 'empty'], true))
                <span class="roster-count">{{ $localeFormatter->number($overview['character_count']) }} / {{ $localeFormatter->number($overview['character_limit']) }}</span>
            @endif
        </div>
        @if ($overview['characters_state'] !== 'empty')<p class="muted">{{ $charactersMessage }}</p>@endif
        @if ($overview['characters_state'] === 'available')
            <div class="character-roster">
                @foreach ($overview['characters'] as $character)
                    <article class="card character-roster-row">
                        <div class="character-monogram" aria-hidden="true">{{ mb_substr($character->name, 0, 1) }}</div>
                        <div class="character-roster-identity">
                            <h3><a href="{{ route('game.characters.show', ['name' => $character->name]) }}">{{ $character->name }}</a></h3>
                            <p>{{ $characterPresentation->vocationName((int) $character->vocation) }} <span aria-hidden="true">·</span> {{ __('portal.account.level') }} {{ $localeFormatter->number($character->level) }}</p>
                        </div>
                        <div class="action-row">
                            <a class="button button-secondary" href="{{ route('game.characters.show', ['name' => $character->name]) }}">{{ __('portal.account.profile') }}</a>
                        </div>
                    </article>
                @endforeach
            </div>
        @elseif ($overview['characters_state'] === 'empty')
            <div class="empty-state"><p>{{ $charactersMessage }}</p>
                @if ($overview['character_creation_allowed'])<a class="button" href="{{ route('account.characters.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.first') }}</a>@endif
            </div>
        @elseif ($overview['characters_state'] === 'unavailable')
            <div class="notice alert-warning" role="status">{{ __('portal.account.unavailable') }}</div>
        @else
            <div class="notice" role="status">{{ __('portal.account.not_ready') }}</div>
        @endif
        @if ($overview['state'] === 'ready' && ! $overview['character_creation_allowed'] && $overview['character_count'] >= $overview['character_limit'])
            <div class="notice alert-warning" role="status">{{ __('portal.account.limit', ['count' => $overview['character_limit']]) }}</div>
        @endif
    </section>
    @endif
    @endif
    <div class="account-services">
        <section aria-labelledby="account-security-heading">
            <p class="eyebrow">{{ __('identity.security.eyebrow') }}</p>
            <h2 id="account-security-heading">{{ __('portal.account.security') }}</h2>
            <dl class="account-facts">
                <div><dt>{{ __('portal.account.email') }}</dt><dd>{{ $identity->email }}</dd></div>
                <div><dt>{{ __('portal.account.mfa') }}</dt><dd><span class="badge {{ $identity->hasConfirmedMfa() ? 'badge-success' : 'badge-warning' }}">{{ $identity->hasConfirmedMfa() ? __('portal.account.enabled') : __('portal.account.not_enabled') }}</span></dd></div>
                <div><dt>{{ __('portal.account.termination') }}</dt><dd><span class="badge {{ $identity->hasPendingTermination() ? 'badge-warning' : 'badge-success' }}">{{ $identity->hasPendingTermination() ? __('portal.account.grace') : __('portal.account.not_requested') }}</span></dd></div>
            </dl>
            <a class="text-link" href="{{ route('identity.account-security.show', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.manage_security') }} <span aria-hidden="true">↗</span></a>
            <div class="action-row"><a href="{{ route('identity.mfa.settings', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.authenticator') }}</a><a href="{{ route('identity.password.change.create', ['locale' => app()->getLocale()]) }}">{{ __('portal.account.password') }}</a></div>
        </section>
        <section aria-labelledby="player-tools-heading">
            <p class="eyebrow">{{ __('portal.account.tools') }}</p>
            <h2 id="player-tools-heading">{{ __('player_companion.account_tool_title') }}</h2>
            <p class="muted">{{ __('player_companion.account_tool_description') }}</p>
            <a class="text-link" href="{{ route('player-companion.session-analyses.index', ['locale' => app()->getLocale()]) }}">{{ __('player_companion.open_tool') }} <span aria-hidden="true">↗</span></a>
        </section>
    </div>
@endsection
