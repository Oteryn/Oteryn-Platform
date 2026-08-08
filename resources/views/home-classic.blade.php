@extends('game.layout')

@section('title', __('public.home.title'))
@section('description', __('public.home.hero_lede'))
@section('page-class', 'portal-home-shell classic-home-shell')

@section('content')
    <section class="portal-hero" aria-labelledby="classic-home-hero-title">
        <div class="portal-hero-art" aria-hidden="true">
            <img src="{{ asset('images/oteryn-hero-citadel.svg') }}" alt="">
        </div>

        <div class="portal-hero-inner">
            <div class="portal-hero-copy">
                <p class="portal-kicker">{{ __('public.home.kicker') }}</p>
                <h1 id="classic-home-hero-title" class="portal-hero-title">{{ __('public.home.hero_title') }}</h1>
                <p class="portal-hero-lede">{{ __('public.home.hero_lede') }}</p>
                <img class="portal-heraldic-divider" src="{{ asset('images/oteryn-heraldic-divider.svg') }}" alt="" aria-hidden="true">
                <div class="portal-hero-actions">
                    @guest
                        <a class="button" href="{{ route('identity.register.create') }}">{{ __('public.account.create') }}</a>
                    @else
                        <a class="button" href="{{ route('account.overview') }}">{{ __('public.home.open_account') }}</a>
                    @endguest
                    <a class="button button-secondary" href="#classic-world-information">{{ __('public.home.view_realm') }}</a>
                </div>
            </div>

            <section class="portal-search-card" aria-labelledby="classic-character-search-heading">
                <div class="portal-search-heading">
                    <img src="{{ asset('images/oteryn-sigil.svg') }}" alt="" aria-hidden="true">
                    <div>
                        <p class="eyebrow">{{ __('public.home.world_activity') }}</p>
                        <h2 id="classic-character-search-heading">{{ __('public.home.find_character') }}</h2>
                    </div>
                </div>
                <form method="GET" action="{{ route('game.characters.search') }}">
                    <div class="form-field">
                        <label for="classic-character-name">{{ __('public.home.character_name') }}</label>
                        <input id="classic-character-name" name="name" type="search" value="{{ old('name') }}" maxlength="255" autocomplete="off" placeholder="{{ __('public.home.character_placeholder') }}" required>
                        <p class="form-help">{{ __('public.home.search_exact') }}</p>
                    </div>
                    @error('name')
                        <p class="notice" role="alert">{{ $message }}</p>
                    @enderror
                    <button type="submit">{{ __('public.home.search') }}</button>
                </form>
            </section>
        </div>
    </section>

    <section id="classic-world-information" class="portal-world-section" aria-labelledby="classic-public-data-heading">
        <div class="portal-world-heading">
            <div class="page-header">
                <p class="eyebrow">{{ __('public.home.discover') }}</p>
                <h2 id="classic-public-data-heading">{{ __('public.home.world_status') }}</h2>
                <p class="muted">{{ __('public.home.continue_journey_help') }}</p>
            </div>
        </div>

        <div class="portal-world-grid">
            <article class="world-card">
                <div class="world-card-header">
                    <img class="world-card-icon" src="{{ asset('images/oteryn-mark-online.svg') }}" alt="" aria-hidden="true">
                    <h3>{{ __('Online') }}</h3>
                </div>
                <p>{{ __('public.home.online_hint') }}</p>
                <a class="world-card-link" href="{{ route('game.online.index') }}">{{ __('Online') }}</a>
            </article>

            <article class="world-card">
                <div class="world-card-header">
                    <img class="world-card-icon" src="{{ asset('images/oteryn-mark-highscores.svg') }}" alt="" aria-hidden="true">
                    <h3>{{ __('Highscores') }}</h3>
                </div>
                <p>{{ __('public.home.highscores_hint') }}</p>
                <a class="world-card-link" href="{{ route('game.highscores.index') }}">{{ __('Highscores') }}</a>
            </article>

            <article class="world-card">
                <div class="world-card-header">
                    <img class="world-card-icon" src="{{ asset('images/oteryn-mark-servers.svg') }}" alt="" aria-hidden="true">
                    <h3>{{ __('Servers') }}</h3>
                </div>
                <p>{{ __('public.home.servers_hint') }}</p>
                <a class="world-card-link" href="{{ route('game.servers.index') }}">{{ __('Servers') }}</a>
            </article>

            <article class="world-card">
                <div class="world-card-header">
                    <img class="world-card-icon" src="{{ asset('images/oteryn-mark-news.svg') }}" alt="" aria-hidden="true">
                    <h3>{{ __('News') }}</h3>
                </div>
                <p>{{ __('public.home.news_hint') }}</p>
                <a class="world-card-link" href="{{ route('news.index') }}">{{ __('News') }}</a>
            </article>
        </div>
    </section>
@endsection
