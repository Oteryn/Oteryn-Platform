@extends('game.layout')

@section('title', 'Homepage design preview')
@section('robots', 'noindex,nofollow')
@section('page-class', 'preview-home-shell')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/home-preview.css') }}">
@endpush

@section('content')
    <aside class="preview-notice" aria-label="Design preview information">
        <span><strong>Design preview.</strong> This page is isolated from the current homepage and does not display invented live statistics.</span>
        <a href="{{ route('home') }}">Open current homepage</a>
    </aside>

    <section class="preview-hero" aria-labelledby="preview-hero-title">
        <div class="preview-hero-art" aria-hidden="true">
            <img src="{{ asset('images/oteryn-hero-citadel.svg') }}" alt="">
        </div>

        <div class="preview-hero-content">
            <div class="preview-hero-copy">
                <p class="preview-kicker">A realm of legends</p>
                <h1 id="preview-hero-title">Answer the call of Oteryn</h1>
                <p class="preview-hero-lede">Ancient powers stir and new heroes rise. Forge alliances, conquer foes and carve your name into the chronicles of Oteryn.</p>

                <div class="preview-hero-actions">
                    @guest
                        <a class="preview-button preview-button-primary" href="{{ route('identity.register.create') }}">Create account</a>
                    @else
                        <a class="preview-button preview-button-primary" href="{{ route('account.overview') }}">Open account center</a>
                    @endguest
                    <a class="preview-button preview-button-secondary" href="#preview-dashboard">View the realm</a>
                </div>
            </div>
        </div>
    </section>

    <section class="preview-search-wrap" aria-labelledby="preview-character-search-heading">
        <div class="preview-search-card">
            <div class="preview-ornament" aria-hidden="true">
                <span></span>
                <img src="{{ asset('images/oteryn-sigil.svg') }}" alt="">
                <span></span>
            </div>
            <h2 id="preview-character-search-heading">Find your character</h2>

            <form class="preview-search-form" method="GET" action="{{ route('game.characters.search') }}">
                <label class="preview-sr-only" for="preview-character-name">Character name</label>
                <span class="preview-search-icon" aria-hidden="true">⌕</span>
                <input id="preview-character-name" name="name" type="search" value="{{ old('name') }}" maxlength="255" autocomplete="off" placeholder="Enter character name..." required>
                <button type="submit">Search character</button>
            </form>

            @error('name')
                <p class="notice" role="alert">{{ $message }}</p>
            @enderror
        </div>
    </section>

    <nav class="preview-realm-strip" aria-label="Realm shortcuts">
        <a href="{{ route('game.online.index') }}">
            <img src="{{ asset('images/oteryn-mark-online.svg') }}" alt="" aria-hidden="true">
            <span><strong>Online now</strong><small>Open the live public view</small></span>
        </a>
        <a href="{{ route('game.highscores.index') }}">
            <img src="{{ asset('images/oteryn-mark-highscores.svg') }}" alt="" aria-hidden="true">
            <span><strong>Highscores</strong><small>Browse level rankings</small></span>
        </a>
        <a href="{{ route('game.servers.index') }}">
            <img src="{{ asset('images/oteryn-mark-servers.svg') }}" alt="" aria-hidden="true">
            <span><strong>Servers</strong><small>Check configured worlds</small></span>
        </a>
        <a href="{{ route('news.index') }}">
            <img src="{{ asset('images/oteryn-mark-news.svg') }}" alt="" aria-hidden="true">
            <span><strong>Latest news</strong><small>Read published updates</small></span>
        </a>
    </nav>

    <section id="preview-dashboard" class="preview-dashboard" aria-label="Oteryn public portal">
        <article class="preview-panel preview-panel-online">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">World activity</p>
                    <h2>Online now</h2>
                </div>
                <span class="preview-live-badge"><span></span>Live view</span>
            </header>

            <div class="preview-online-emblem" aria-hidden="true">
                <span>O</span>
            </div>
            <p>Review characters represented by the current public online data boundary.</p>
            <a class="preview-panel-link" href="{{ route('game.online.index') }}">View online characters</a>
        </article>

        <article class="preview-panel preview-panel-highscores">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">Hall of heroes</p>
                    <h2>Highscores</h2>
                </div>
                <img src="{{ asset('images/oteryn-mark-highscores.svg') }}" alt="" aria-hidden="true">
            </header>

            <ol class="preview-ranking-list">
                <li><span>Level rankings</span><small>Active characters</small></li>
                <li><span>Deterministic order</span><small>Public fields only</small></li>
                <li><span>Bounded pages</span><small>Open the full ranking</small></li>
            </ol>
            <a class="preview-panel-link" href="{{ route('game.highscores.index') }}">View full highscores</a>
        </article>

        <article class="preview-panel preview-panel-servers">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">Realm access</p>
                    <h2>Servers</h2>
                </div>
                <img src="{{ asset('images/oteryn-mark-servers.svg') }}" alt="" aria-hidden="true">
            </header>

            <div class="preview-server-lines" aria-hidden="true">
                <span><i></i><i></i><i></i><i></i><i></i></span>
                <span><i></i><i></i><i></i><i></i></span>
                <span><i></i><i></i><i></i></span>
            </div>
            <p>See configured channels and available runtime information on the existing authoritative server page.</p>
            <a class="preview-panel-link" href="{{ route('game.servers.index') }}">View all servers</a>
        </article>

        <article class="preview-panel preview-panel-news">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">Chronicles</p>
                    <h2>Latest news</h2>
                </div>
                <img src="{{ asset('images/oteryn-mark-news.svg') }}" alt="" aria-hidden="true">
            </header>

            <div class="preview-card-art preview-card-art-news" aria-hidden="true"></div>
            <h3>Published Oteryn updates</h3>
            <p>Open the current news archive for published Platform announcements. This design preview does not invent an article title or publication date.</p>
            <a class="preview-panel-link" href="{{ route('news.index') }}">Read the latest news</a>
        </article>

        <article class="preview-panel preview-panel-feature">
            <header class="preview-panel-heading">
                <div>
                    <p class="preview-panel-kicker">Featured region</p>
                    <h2>Explore the realm</h2>
                </div>
                <img src="{{ asset('images/oteryn-sigil.svg') }}" alt="" aria-hidden="true">
            </header>

            <div class="preview-card-art preview-card-art-realm" aria-hidden="true"></div>
            <h3>The world of Oteryn</h3>
            <p>A visual space reserved for a future verified region feature, event or managed CMS campaign.</p>
            <a class="preview-panel-link" href="{{ route('game.servers.index') }}">Explore public world data</a>
        </article>
    </section>
@endsection
