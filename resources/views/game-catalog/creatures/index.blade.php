@extends('game.layout')

@section('title', __('game_catalog.creatures.title'))
@section('description', __('game_catalog.creatures.intro'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow"><a href="{{ route('game-catalog.index') }}">{{ __('game_catalog.title') }}</a></p>
        <h1>{{ __('game_catalog.creatures.title') }}</h1>
        <p class="muted">{{ __('game_catalog.creatures.intro') }}</p>
    </header>

    @include('game-catalog.partials.context')

    @if ($context !== null)
        <form class="catalog-filters" method="get" action="{{ route('game-catalog.creatures.index') }}" aria-label="{{ __('game_catalog.filters.label') }}">
            <label>
                <span>{{ __('game_catalog.filters.search') }}</span>
                <input type="search" name="q" maxlength="100" value="{{ $filters['q'] }}">
            </label>
            <label>
                <span>{{ __('game_catalog.creatures.boss_status') }}</span>
                <select name="boss">
                    <option value="">{{ __('game_catalog.filters.all') }}</option>
                    <option value="1" @selected($filters['boss'] === true)>{{ __('game_catalog.yes') }}</option>
                    <option value="0" @selected($filters['boss'] === false)>{{ __('game_catalog.no') }}</option>
                </select>
            </label>
            <label>
                <span>{{ __('game_catalog.creatures.bestiary_class') }}</span>
                <select name="bestiary_class">
                    <option value="">{{ __('game_catalog.filters.all') }}</option>
                    @foreach ($bestiaryClasses as $bestiaryClass)
                        <option value="{{ $bestiaryClass }}" @selected($filters['bestiary_class'] === $bestiaryClass)>{{ $bestiaryClass }}</option>
                    @endforeach
                </select>
            </label>
            <button type="submit">{{ __('game_catalog.filters.apply') }}</button>
            <a class="button-link" href="{{ route('game-catalog.creatures.index') }}">{{ __('game_catalog.filters.clear') }}</a>
        </form>

        @if ($creatures->isEmpty())
            <div class="empty-state" role="status">
                <strong>{{ __('game_catalog.creatures.empty') }}</strong>
                <p>{{ __('game_catalog.creatures.empty_help') }}</p>
            </div>
        @else
            <p class="catalog-result-count">{{ trans_choice('game_catalog.creatures.result_count', $creatures->total(), ['count' => $creatures->total()]) }}</p>
            <div class="catalog-card-grid">
                @foreach ($creatures as $creature)
                    <article class="catalog-card">
                        <div class="catalog-image-placeholder" aria-hidden="true"><span>?</span></div>
                        <div>
                            <p class="catalog-kicker">{{ $creature->bestiary_class ?? __('game_catalog.unknown') }}@if($creature->is_boss) · {{ __('game_catalog.creatures.boss') }}@endif</p>
                            <h2><a href="{{ route('game-catalog.creatures.show', ['slug' => \Illuminate\Support\Str::after($creature->canonical_key, ':')]) }}">{{ $creature->name }}</a></h2>
                            <dl class="catalog-compact-stats">
                                <div><dt>{{ __('game_catalog.creatures.health') }}</dt><dd>{{ $creature->health }}</dd></div>
                                <div><dt>{{ __('game_catalog.creatures.experience') }}</dt><dd>{{ $creature->experience }}</dd></div>
                                <div><dt>{{ __('game_catalog.creatures.armor') }}</dt><dd>{{ $creature->armor }}</dd></div>
                                <div><dt>{{ __('game_catalog.creatures.availability') }}</dt><dd>{{ $creature->availability }}</dd></div>
                            </dl>
                        </div>
                    </article>
                @endforeach
            </div>
            <div class="catalog-pagination">{{ $creatures->links() }}</div>
        @endif
    @endif
@endsection
