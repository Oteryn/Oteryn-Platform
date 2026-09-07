@extends('game.layout')

@section('title', __('public.home.title'))
@section('description', __('public.home.hero_lede'))
@section('page-class', 'production-home-shell')
@section('portal-family', 'home')

@push('head')
    <link rel="preload" as="image" href="{{ asset('images/oteryn-citadel.webp') }}" fetchpriority="high">
@endpush
@push('styles')
    <link rel="stylesheet" href="{{ asset('css/home-production.css') }}">
@endpush

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <section class="realm-hero" aria-labelledby="home-hero-title">
        <img class="realm-hero-art" src="{{ asset('images/oteryn-citadel.webp') }}" width="626" height="468" alt="" aria-hidden="true" fetchpriority="high">
        <div class="realm-hero-copy">
            <p class="eyebrow">{{ __('public.home.kicker') }}</p>
            <h1 id="home-hero-title" aria-label="Oteryn Platform">OTERYN</h1>
            <h2 class="realm-hero-tagline">{{ __('public.home.hero_title') }}</h2>
            <p class="realm-hero-lede">{{ __('public.home.hero_lede') }}</p>
            <div class="realm-hero-actions">
                @guest
                    <a class="button" href="{{ route('identity.register.create', ['locale' => app()->getLocale()]) }}">{{ __('public.account.create') }} <span aria-hidden="true">↗</span></a>
                    <a class="button button-secondary" href="{{ route('identity.login.create', ['locale' => app()->getLocale()]) }}">{{ __('public.account.sign_in') }}</a>
                @else
                    <a class="button" href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}">{{ __('public.home.open_account') }} <span aria-hidden="true">↗</span></a>
                    <a class="button button-secondary" href="{{ route('account.characters.create', ['locale' => app()->getLocale()]) }}">{{ __('public.home.create_character') }}</a>
                @endguest
            </div>
        </div>
    </section>

    <section class="realm-pulse production-hero-world" data-hero-world-state="{{ $homePage->world->state->value }}" aria-label="{{ __('public.home.world_activity') }}">
        <div class="portal-container realm-pulse-inner">
            <span class="production-state-badge production-state-{{ strtolower($homePage->world->state->value) }}">{{ __('public.states.'.strtolower($homePage->world->state->value)) }}</span>
            @if ($homePage->world->state === \App\PublicPortal\PublicContentState::AVAILABLE)
                <strong>{{ trans_choice('public.home.players_online', $homePage->world->playersOnline ?? 0, ['count' => $localeFormatter->number($homePage->world->playersOnline ?? 0)]) }}</strong>
            @else
                <p>{{ __('portal.home.summary_'.strtolower($homePage->world->state->value)) }}</p>
            @endif
            @if (collect($homePage->world->channels)->contains(static fn ($channel) => $channel->maintenance))
                <p class="production-hero-maintenance">{{ __('public.home.maintenance') }}</p>
            @endif
            <a href="#realm-overview">{{ __('public.home.view_realm') }} <span aria-hidden="true">↓</span></a>
        </div>
    </section>

    <section id="character-search" class="realm-search portal-container" aria-labelledby="home-character-search-heading">
        <div>
            <p class="eyebrow">{{ __('public.home.meet_heroes') }}</p>
            <h2 id="home-character-search-heading">{{ __('public.home.find_character') }}</h2>
        </div>
        <form class="realm-search-form" method="GET" action="{{ route('game.characters.search') }}">
            <label class="portal-sr-only" for="home-character-name">{{ __('public.home.character_name') }}</label>
            <input id="home-character-name" name="name" type="search" value="{{ old('name') }}" maxlength="255" autocomplete="off" placeholder="{{ __('public.home.character_placeholder') }}" required aria-describedby="home-character-help @error('name') home-character-error @enderror" @error('name') aria-invalid="true" @enderror>
            <button type="submit">{{ __('public.home.search') }} <span aria-hidden="true">→</span></button>
            <p id="home-character-help" class="form-help">{{ __('public.home.search_exact') }}</p>
            @error('name')
                <p id="home-character-error" class="form-error" role="alert">{{ $message }}</p>
            @enderror
        </form>
        <nav class="realm-quick-links" aria-label="{{ __('portal.home.tools') }}">
            <a href="{{ route('game.highscores.index') }}">{{ __('Highscores') }} <span aria-hidden="true">↗</span></a>
            <a href="{{ route('game.online.index') }}">{{ __('Online') }} <span aria-hidden="true">↗</span></a>
            <a href="{{ route('game.guilds.index') }}">{{ __('public.navigation.guilds') }} <span aria-hidden="true">↗</span></a>
        </nav>
    </section>

    <div class="realm-journal portal-container">
        <section class="production-news-panel" data-content-state="{{ $homePage->news->state->value }}" aria-labelledby="home-news-heading">
            <header class="section-heading">
                <p class="eyebrow">{{ __('portal.home.chronicles') }}</p>
                <h2 id="home-news-heading">{{ __('public.home.latest_news') }}</h2>
                <a class="realm-text-link" href="{{ route('news.index') }}">{{ __('public.home.read_news') }} <span aria-hidden="true">→</span></a>
            </header>
            @if ($homePage->news->state === \App\PublicPortal\PublicContentState::AVAILABLE)
                <div class="production-news-list">
                    @foreach ($homePage->news->posts as $post)
                        <article class="journal-story {{ $loop->first ? 'journal-story-lead' : '' }}">
                            <p class="production-news-date">{{ $post->published_at ? $localeFormatter->date($post->published_at) : '' }}</p>
                            <h3><a href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ $post->title }}</a></h3>
                            <p>{{ Str::limit($post->body, $loop->first ? 230 : 150) }}</p>
                            <a class="realm-text-link" href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ __('public.news.read') }} <span aria-hidden="true">→</span></a>
                        </article>
                    @endforeach
                </div>
            @elseif ($homePage->news->state === \App\PublicPortal\PublicContentState::EMPTY)
                <div class="empty-state" role="status">{{ __('public.home.no_news') }}</div>
            @else
                <div class="notice" role="status">{{ __('public.home.news_unavailable') }}</div>
            @endif
        </section>

        <aside id="realm-overview" class="realm-ledger" data-content-state="{{ $homePage->world->state->value }}" aria-labelledby="realm-overview-heading">
            <header class="section-heading">
                <p class="eyebrow">{{ __('public.home.world_activity') }}</p>
                <h2 id="realm-overview-heading">{{ __('public.home.world_status') }}</h2>
            </header>
            @switch($homePage->world->state)
                @case(\App\PublicPortal\PublicContentState::AVAILABLE)
                    <p class="production-world-total"><strong>{{ $localeFormatter->number($homePage->world->playersOnline ?? 0) }}</strong><span>{{ __('public.game.players_online') }}</span></p>
                    @break
                @case(\App\PublicPortal\PublicContentState::EMPTY)
                    <div class="empty-state" role="status">{{ __('public.home.no_worlds') }}</div>
                    @break
                @case(\App\PublicPortal\PublicContentState::STALE)
                    <div class="notice alert-warning" role="status">{{ __('public.home.world_stale') }}</div>
                    @break
                @case(\App\PublicPortal\PublicContentState::UNAVAILABLE)
                    <div class="notice" role="status">{{ __('public.home.world_unavailable') }}</div>
                    @break
            @endswitch
            <div class="production-world-list">
                @foreach ($homePage->world->channels as $channel)
                    <section class="production-world-row" aria-label="{{ __('public.home.world_status_label', ['world' => $channel->name]) }}">
                        <div>
                            <h3>{{ $channel->name }}</h3>
                            <p>{{ $channel->pvpType }} · {{ __('public.home.capacity', ['count' => $localeFormatter->number($channel->maxPlayers)]) }}</p>
                        </div>
                        <div class="production-runtime-summary">
                            @if ($homePage->world->state === \App\PublicPortal\PublicContentState::UNAVAILABLE)
                                <strong>{{ __('public.states.unavailable') }}</strong>
                            @elseif ($channel->runtimeStatus === null)
                                <strong>{{ __('public.states.stale') }}</strong>
                            @else
                                <strong>{{ $channel->runtimeStatus }}</strong>
                                <span>{{ __('public.home.online_count', ['count' => $localeFormatter->number($channel->playersOnline ?? 0)]) }}</span>
                            @endif
                        </div>
                        @if ($channel->maintenance)
                            <p class="production-maintenance"><strong>{{ __('public.home.maintenance') }}</strong>@if ($channel->maintenanceMessage) {{ $channel->maintenanceMessage }}@endif</p>
                        @endif
                    </section>
                @endforeach
            </div>
            <a class="realm-text-link" href="{{ route('game.servers.index') }}">{{ __('public.home.view_servers') }} <span aria-hidden="true">→</span></a>
        </aside>
    </div>

    <section class="realm-dispatches portal-container" aria-label="{{ __('public.home.community_updates') }}">
        @include('announcements.components.ticker', ['ticker' => $homePage->announcements])
        @include('events.components.upcoming-summary', ['summary' => $homePage->upcomingEvent])
    </section>

    <section class="realm-journey" aria-labelledby="home-journey-heading">
        <div class="portal-container realm-journey-inner">
            <header>
                <p class="eyebrow">{{ __('public.home.begin_journey') }}</p>
                <h2 id="home-journey-heading">{{ __('portal.home.journey') }}</h2>
            </header>
            <ol class="production-path-list">
                @guest
                    <li><a href="{{ route('identity.register.create', ['locale' => app()->getLocale()]) }}"><strong>{{ __('public.account.create') }}</strong><span>{{ __('public.home.create_account_help') }}</span></a></li>
                @else
                    <li><a href="{{ route('account.overview', ['locale' => app()->getLocale()]) }}"><strong>{{ __('public.account.center') }}</strong><span>{{ __('public.home.account_help') }}</span></a></li>
                @endguest
                <li><a href="{{ route('downloads.index') }}"><strong>{{ __('public.downloads.title') }}</strong><span>{{ __('public.home.download_help') }}</span></a></li>
                <li><a href="{{ route('editorial.getting-started') }}"><strong>{{ __('Beginner\'s Guide') }}</strong><span>{{ __('public.home.guide_help') }}</span></a></li>
            </ol>
        </div>
    </section>

    <section class="production-discover portal-container" aria-labelledby="home-discover-title">
        <header class="section-heading">
            <p class="eyebrow">{{ __('public.home.discover') }}</p>
            <h2 id="home-discover-title">{{ __('public.home.continue_journey') }}</h2>
        </header>
        <div class="production-discover-grid">
            @foreach ([
                ['downloads.index', 'public.downloads.title', 'public.home.download_help'],
                ['editorial.getting-started', 'Beginner\'s Guide', 'public.home.guide_help'],
                ['wiki.index', 'public.wiki.title', 'public.home.wiki_help'],
                ['events.index', 'public.events.title', 'public.home.events_help'],
                ['game.guilds.index', 'public.game.guild_directory', 'public.home.guilds_help'],
                ['support.index', 'support.nav.support_center', 'public.home.support_help'],
            ] as [$routeName, $label, $help])
                <a href="{{ route($routeName) }}"><strong>{{ __($label) }}</strong><span>{{ __($help) }}</span><span class="discover-arrow" aria-hidden="true">↗</span></a>
            @endforeach
        </div>
    </section>
@endsection
