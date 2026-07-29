@extends('game.layout')

@section('title', __('game_catalog.title'))
@section('description', __('game_catalog.description'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow">{{ __('game_catalog.eyebrow') }}</p>
        <h1>{{ __('game_catalog.title') }}</h1>
        <p class="muted">{{ __('game_catalog.description') }}</p>
        @if ($catalog !== null)
            <div class="catalog-meta">
                <span class="catalog-chip">{{ __('game_catalog.active_content', ['profile' => $catalog->context->profileName]) }}</span>
                <span class="catalog-chip">{{ __('game_catalog.target_release', ['release' => $catalog->context->targetRelease]) }}</span>
            </div>
        @endif
    </header>

    @if ($catalog === null)
        <div class="empty-state" role="status">
            <strong>{{ __('game_catalog.not_active') }}</strong>
            <p>{{ __('game_catalog.not_active_help') }}</p>
        </div>
    @else
        <section class="catalog-grid" aria-label="{{ __('game_catalog.title') }}">
            <article class="card catalog-card">
                <p class="eyebrow">{{ __('game_catalog.item_count', ['count' => $catalog->itemCount]) }}</p>
                <h2>{{ __('game_catalog.items') }}</h2>
                <p>{{ __('game_catalog.search_placeholder_items') }}</p>
                <p><a class="button" href="{{ route('game-catalog.items.index') }}">{{ __('game_catalog.browse_items') }}</a></p>
            </article>
            <article class="card catalog-card">
                <p class="eyebrow">{{ __('game_catalog.creature_count', ['count' => $catalog->creatureCount]) }}</p>
                <h2>{{ __('game_catalog.creatures') }}</h2>
                <p>{{ __('game_catalog.search_placeholder_creatures') }}</p>
                <p><a class="button" href="{{ route('game-catalog.creatures.index') }}">{{ __('game_catalog.browse_creatures') }}</a></p>
            </article>
        </section>
    @endif
@endsection
