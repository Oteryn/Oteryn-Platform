@extends('game.layout')

@section('title', __('public.home.title'))
@section('description', __('public.home.hero_lede'))
@section('page-class', 'preview-home-shell production-home-shell')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/home-preview.css') }}">
    <link rel="stylesheet" href="{{ asset('css/home-production.css') }}">
@endpush

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <section class="preview-hero" aria-labelledby="home-hero-title">
        <div class="preview-hero-art" aria-hidden="true">
            <img src="{{ asset('images/oteryn-hero-citadel.svg') }}" alt="">
        </div>

        <div class="preview-hero-content">
            <div class="preview-hero-copy">
                <p class="preview-kicker">{{ __('public.home.kicker') }}</p>
                <h1 id="home-hero-title"><span class="preview-sr-only">Oteryn Platform. </span>{{ __('public.home.hero_title') }}</h1>
                <p class="preview-hero-lede">{{ __('public.home.hero_lede') }}</p>

                <div class="preview-hero-actions">
                    @guest
                        <a class="preview-button preview-button-primary" href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
                        <a class="preview-button preview-button-secondary" href="{{ route('identity.login.create') }}">{{ __('public.account.sign_in') }}</a>
                    @else
                        <a class="preview-button preview-button-primary" href="{{ route('account.overview') }}">{{ __('public.home.open_account') }}</a>
                    @endguest
                    <a class="preview-button preview-button-secondary" href="#realm-overview">{{ __('public.home.view_realm') }}</a>
                </div>
            </div>
        </div>
    </section>

    <section id="character-search" class="preview-search-wrap" aria-labelledby="home-character-search-heading">
        <div class="preview-search-card">
            <div class="preview-ornament" aria-hidden="true">
                <span></span>
                <img src="{{ asset('images/oteryn-sigil.svg') }}" alt="">
                <span></span>
            </div>
            <h2 id="home-character-search-heading">{{ __('public.home.find_character') }}</h2>
            <p class="preview-sr-only">{{ __('public.home.search_exact') }}</p>

            <form class="preview-search-form" method="GET" action="{{ route('game.characters.search') }}">
                <label class="preview-sr-only" for="home-character-name">{{ __('public.home.character_name') }}</label>
                <span class="preview-search-icon" aria-hidden="true">⌕</span>
                <input id="home-character-name" name="name" type="search" value="{{ old('name') }}" maxlength="255" autocomplete="off" placeholder="{{ __('public.home.character_placeholder') }}" required>
                <button type="submit">{{ __('public.home.search') }}</button>
            </form>

            @error('name')
                <p class="notice" role="alert">{{ $message }}</p>
            @enderror
        </div>
    </section>

    <nav class="preview-realm-strip" aria-label="{{ __('public.home.realm_shortcuts') }}">
        <a href="{{ route('game.online.index') }}">
            <img src="{{ asset('images/oteryn-mark-online.svg') }}" alt="" aria-hidden="true">
            <span><strong>{{ __('Online') }}</strong><small>{{ __('public.home.online_hint') }}</small></span>
        </a>
        <a href="{{ route('game.highscores.index') }}">
            <img src="{{ asset('images/oteryn-mark-highscores.svg') }}" alt="" aria-hidden="true">
            <span><strong>{{ __('Highscores') }}</strong><small>{{ __('public.home.highscores_hint') }}</small></span>
        </a>
        <a href="{{ route('game.servers.index') }}">
            <img src="{{ asset('images/oteryn-mark-servers.svg') }}" alt="" aria-hidden="true">
            <span><strong>{{ __('Servers') }}</strong><small>{{ __('public.home.servers_hint') }}</small></span>
        </a>
        <a href="{{ route('news.index') }}">
            <img src="{{ asset('images/oteryn-mark-news.svg') }}" alt="" aria-hidden="true">
            <span><strong>{{ __('News') }}</strong><small>{{ __('public.home.news_hint') }}</small></span>
        </a>
    </nav>

    <section id="realm-overview" class="production-dashboard" aria-label="{{ __('public.home.portal_label') }}">
        <article class="preview-panel production-world-panel" data-content-state="{{ $homePage->world->state->value }}">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">{{ __('public.home.world_activity') }}</p>
                    <h2>{{ __('public.home.world_status') }}</h2>
                </div>
                <span class="production-state-badge production-state-{{ strtolower($homePage->world->state->value) }}">{{ __('public.states.'.strtolower($homePage->world->state->value)) }}</span>
            </header>

            @switch($homePage->world->state)
                @case(\App\PublicPortal\PublicContentState::AVAILABLE)
                    <p class="production-world-total"><strong>{{ trans_choice('public.home.players_online', $homePage->world->playersOnline ?? 0, ['count' => $localeFormatter->number($homePage->world->playersOnline ?? 0)]) }}</strong></p>
                    @break
                @case(\App\PublicPortal\PublicContentState::EMPTY)
                    <div class="production-state-message" role="status">{{ __('public.home.no_worlds') }}</div>
                    @break
                @case(\App\PublicPortal\PublicContentState::STALE)
                    <div class="production-state-message" role="status">{{ __('public.home.world_stale') }}</div>
                    @break
                @case(\App\PublicPortal\PublicContentState::UNAVAILABLE)
                    <div class="production-state-message" role="status">{{ __('public.home.world_unavailable') }}</div>
                    @break
            @endswitch

            @if ($homePage->world->channels !== [])
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
            @endif

            <a class="preview-panel-link" href="{{ route('game.servers.index') }}">{{ __('public.home.view_servers') }}</a>
        </article>

        <article class="preview-panel production-news-panel" data-content-state="{{ $homePage->news->state->value }}">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">{{ __('public.home.chronicles') }}</p>
                    <h2>{{ __('public.home.latest_news') }}</h2>
                </div>
                <span class="production-state-badge production-state-{{ strtolower($homePage->news->state->value) }}">{{ __('public.states.'.strtolower($homePage->news->state->value)) }}</span>
            </header>

            @if ($homePage->news->state === \App\PublicPortal\PublicContentState::AVAILABLE)
                <div class="production-news-list">
                    @foreach ($homePage->news->posts as $post)
                        <article>
                            <p class="production-news-date">{{ $post->published_at ? $localeFormatter->date($post->published_at) : '' }}</p>
                            <h3><a href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ $post->title }}</a></h3>
                            <p>{{ Str::limit($post->body, 170) }}</p>
                        </article>
                    @endforeach
                </div>
            @elseif ($homePage->news->state === \App\PublicPortal\PublicContentState::EMPTY)
                <div class="production-state-message" role="status">{{ __('public.home.no_news') }}</div>
            @else
                <div class="production-state-message" role="status">{{ __('public.home.news_unavailable') }}</div>
            @endif

            <a class="preview-panel-link" href="{{ route('news.index') }}">{{ __('public.home.read_news') }}</a>
        </article>

        <article class="preview-panel production-path-panel">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">{{ __('public.home.begin_journey') }}</p>
                    <h2>{{ __('public.home.your_path') }}</h2>
                </div>
                <img src="{{ asset('images/oteryn-sigil.svg') }}" alt="" aria-hidden="true">
            </header>
            <div class="production-path-list">
                @guest
                    <a href="{{ route('identity.register.create') }}"><strong>{{ __('public.account.create') }}</strong><span>{{ __('public.home.create_account_help') }}</span></a>
                    <a href="{{ route('identity.login.create') }}"><strong>{{ __('public.account.sign_in') }}</strong><span>{{ __('public.home.sign_in_help') }}</span></a>
                @else
                    <a href="{{ route('account.overview') }}"><strong>{{ __('public.account.center') }}</strong><span>{{ __('public.home.account_help') }}</span></a>
                    <a href="{{ route('account.characters.create') }}"><strong>{{ __('public.home.create_character') }}</strong><span>{{ __('public.home.create_character_help') }}</span></a>
                @endguest
                <a href="{{ route('game.highscores.index') }}"><strong>{{ __('public.home.meet_heroes') }}</strong><span>{{ __('public.home.meet_heroes_help') }}</span></a>
            </div>
        </article>
    </section>

    <section class="production-community-grid" aria-label="{{ __('public.home.community_updates') }}">
        @include('announcements.components.ticker', ['ticker' => $homePage->announcements])
        @include('events.components.upcoming-summary', ['summary' => $homePage->upcomingEvent])
    </section>

    <section class="production-discover" aria-labelledby="home-discover-title">
        <div class="section-heading">
            <p class="eyebrow">{{ __('public.home.discover') }}</p>
            <h2 id="home-discover-title">{{ __('public.home.continue_journey') }}</h2>
            <p class="muted">{{ __('public.home.continue_journey_help') }}</p>
        </div>
        <div class="production-discover-grid">
            <a class="card" href="{{ route('downloads.index') }}">
                <strong>{{ __('public.downloads.title') }}</strong>
                <span>{{ __('public.home.download_help') }}</span>
            </a>
            <a class="card" href="{{ route('editorial.getting-started') }}">
                <strong>{{ __('Beginner\'s Guide') }}</strong>
                <span>{{ __('public.home.guide_help') }}</span>
            </a>
            <a class="card" href="{{ route('wiki.index') }}">
                <strong>{{ __('public.wiki.title') }}</strong>
                <span>{{ __('public.home.wiki_help') }}</span>
            </a>
            <a class="card" href="{{ route('events.index') }}">
                <strong>{{ __('public.events.title') }}</strong>
                <span>{{ __('public.home.events_help') }}</span>
            </a>
            <a class="card" href="{{ route('game.guilds.index') }}">
                <strong>{{ __('public.game.guild_directory') }}</strong>
                <span>{{ __('public.home.guilds_help') }}</span>
            </a>
            <a class="card" href="{{ route('support.index') }}">
                <strong>{{ __('support.nav.support_center') }}</strong>
                <span>{{ __('public.home.support_help') }}</span>
            </a>
        </div>
    </section>
@endsection
