@extends('game.layout')

@section('title', $character->name)
@section('page-class', 'page-shell-wide community-page')

@section('portal-family', 'world-data')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    @inject('characterPresentation', 'App\PublicGameData\CharacterPresentation')

    <header class="page-header player-masthead">
        <div class="player-emblem player-emblem-large" aria-hidden="true">{{ mb_substr($character->name, 0, 1) }}</div>
        <div class="player-masthead-copy">
        <p class="eyebrow">{{ __('community.profile.title') }}</p>
        <h1>{{ $character->name }}</h1>
        <p class="muted">{{ __('community.profile.description') }}</p>
        </div>
    </header>

    <section aria-labelledby="character-overview-heading">
        <h2 id="character-overview-heading">{{ __('community.profile.overview') }}</h2>
        <dl class="stat-grid">
            <div class="stat">
                <dt>{{ __('public.game.level') }}</dt>
                <dd class="display-title">{{ $localeFormatter->number($character->level) }}</dd>
            </div>
            <div class="stat" data-vocation-id="{{ $character->vocation }}">
                <dt>{{ __('public.game.vocation') }}</dt>
                <dd>{{ $characterPresentation->publishedVocationName($character->vocation) }}</dd>
            </div>
            @if ($character->guild_visible && ($character->guild_name !== null || ! $character->guild_member))
                <div class="stat">
                    <dt>{{ __('public.game.guild') }}</dt>
                    <dd>
                        @if ($character->guild_name !== null)
                            {{ $character->guild_name }}
                            @if ($character->guild_rank !== null)
                                <span class="muted">· {{ $character->guild_rank }}</span>
                            @endif
                        @else
                            {{ __('game.profile.no_guild') }}
                        @endif
                    </dd>
                </div>
            @endif
        </dl>
    </section>
@endsection
