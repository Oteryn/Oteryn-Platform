@extends('game.layout')

@section('title', __('game_catalog.creatures'))
@section('description', __('game_catalog.search_placeholder_creatures'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow"><a href="{{ route('game-catalog.index') }}">{{ __('game_catalog.title') }}</a></p>
        <h1>{{ __('game_catalog.creatures') }}</h1>
        @if ($catalog !== null)
            <div class="catalog-meta">
                <span class="catalog-chip">{{ __('game_catalog.target_release', ['release' => $catalog->context->targetRelease]) }}</span>
                <span class="catalog-chip">{{ __('game_catalog.results', ['count' => $catalog->total]) }}</span>
            </div>
        @endif
    </header>

    @if ($catalog === null)
        <div class="empty-state" role="status">
            <strong>{{ __('game_catalog.not_active') }}</strong>
            <p>{{ __('game_catalog.not_active_help') }}</p>
        </div>
    @else
        <form class="catalog-filters" method="get" action="{{ route('game-catalog.creatures.index') }}">
            <label class="catalog-field">
                <span>{{ __('game_catalog.search') }}</span>
                <input type="search" name="q" value="{{ $catalog->query }}" maxlength="80" placeholder="{{ __('game_catalog.search_placeholder_creatures') }}">
            </label>
            <label class="catalog-field">
                <span>{{ __('game_catalog.bestiary_class') }}</span>
                <select name="bestiary_class">
                    <option value="">{{ __('game_catalog.all_bestiary_classes') }}</option>
                    @foreach ($catalog->bestiaryClasses as $bestiaryClass)
                        <option value="{{ $bestiaryClass }}" @selected($catalog->bestiaryClass === $bestiaryClass)>{{ $bestiaryClass }}</option>
                    @endforeach
                </select>
            </label>
            <label class="catalog-checkbox">
                <input type="checkbox" name="boss" value="1" @checked($catalog->bossOnly)>
                <span>{{ __('game_catalog.bosses_only') }}</span>
            </label>
            <div>
                <button type="submit">{{ __('game_catalog.filter') }}</button>
                <a href="{{ route('game-catalog.creatures.index') }}">{{ __('game_catalog.clear_filters') }}</a>
            </div>
        </form>

        @if ($catalog->creatures === [])
            <div class="empty-state" role="status"><strong>{{ __('game_catalog.empty_creatures') }}</strong></div>
        @else
            <section class="catalog-grid" aria-label="{{ __('game_catalog.creatures') }}">
                @foreach ($catalog->creatures as $creature)
                    <article class="card catalog-card">
                        <p class="eyebrow">{{ $creature->bestiaryClass ?? __('game_catalog.creatures') }}@if ($creature->boss) · {{ __('game_catalog.boss') }}@endif</p>
                        <h2><a href="{{ route('game-catalog.creatures.show', ['slug' => $creature->slug]) }}">{{ $creature->name }}</a></h2>
                        @if ($creature->summary !== null && $creature->summary !== '')<p>{{ $creature->summary }}</p>@endif
                        <dl class="catalog-stats">
                            <div><dt>{{ __('game_catalog.health') }}</dt><dd>{{ $creature->health }}</dd></div>
                            <div><dt>{{ __('game_catalog.experience') }}</dt><dd>{{ $creature->experience }}</dd></div>
                        </dl>
                    </article>
                @endforeach
            </section>
        @endif

        @if ($catalog->lastPage() > 1)
            <nav class="catalog-pagination" aria-label="{{ __('game_catalog.page', ['current' => $catalog->page, 'last' => $catalog->lastPage()]) }}">
                @if ($catalog->page > 1)
                    <a class="button" href="{{ route('game-catalog.creatures.index', array_merge(request()->only(['q', 'bestiary_class', 'boss']), ['page' => $catalog->page - 1])) }}">{{ __('game_catalog.previous') }}</a>
                @else
                    <span></span>
                @endif
                <span>{{ __('game_catalog.page', ['current' => $catalog->page, 'last' => $catalog->lastPage()]) }}</span>
                @if ($catalog->page < $catalog->lastPage())
                    <a class="button" href="{{ route('game-catalog.creatures.index', array_merge(request()->only(['q', 'bestiary_class', 'boss']), ['page' => $catalog->page + 1])) }}">{{ __('game_catalog.next') }}</a>
                @endif
            </nav>
        @endif
    @endif
@endsection
