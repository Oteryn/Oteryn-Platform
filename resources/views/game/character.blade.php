@extends('game.layout')

@section('title', $character->name)

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')

    <header class="page-header">
        <p class="eyebrow">{{ __('game.profile.eyebrow') }}</p>
        <h1>{{ $character->name }}</h1>
        <p class="muted">{{ __('game.profile.description') }}</p>
    </header>

    <dl class="stat-grid" aria-label="{{ __('game.profile.eyebrow') }}">
        <div class="stat">
            <dt>{{ __('public.game.level') }}</dt>
            <dd class="display-title">{{ $localeFormatter->number($character->level) }}</dd>
        </div>
        <div class="stat">
            <dt>{{ __('public.game.vocation') }}</dt>
            <dd>{{ $characterPresentation->vocationName((int) $character->vocation) }}</dd>
        </div>
        <div class="stat">
            <dt>{{ __('public.game.guild') }}</dt>
            <dd>
                @if (is_string($character->guild_name) && $character->guild_name !== '')
                    <a href="{{ route('game.guilds.show', ['name' => $character->guild_name]) }}">
                        {{ $character->guild_name }}
                    </a>
                @else
                    {{ __('game.profile.no_guild') }}
                @endif
            </dd>
        </div>
    </dl>

    <section class="card" aria-labelledby="character-explore-heading">
        <p class="eyebrow">{{ __('game.profile.explore') }}</p>
        <h2 id="character-explore-heading">{{ __('public.game.community') }}</h2>
        <div class="action-row">
            <a class="button button-secondary" href="{{ route('game.highscores.index') }}">
                {{ __('game.profile.highscores') }}
            </a>
            <a class="button button-secondary" href="{{ route('game.online.index') }}">
                {{ __('game.profile.online') }}
            </a>
        </div>
    </section>
@endsection
