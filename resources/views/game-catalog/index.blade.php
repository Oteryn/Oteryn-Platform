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
    </header>

    @include('game-catalog.partials.context')

    @if ($context !== null)
        <div class="catalog-surface-grid">
            <article class="catalog-surface-card">
                <h2><a href="{{ route('game-catalog.items.index') }}">{{ __('game_catalog.items.title') }}</a></h2>
                <p>{{ __('game_catalog.items.intro') }}</p>
                @if ($categories !== [])
                    <ul class="catalog-tag-list" aria-label="{{ __('game_catalog.items.categories') }}">
                        @foreach (array_slice($categories, 0, 8) as $category)
                            <li><a href="{{ route('game-catalog.items.index', ['category' => $category->category]) }}">{{ $category->category }} <span aria-hidden="true">({{ $category->item_count }})</span></a></li>
                        @endforeach
                    </ul>
                @endif
            </article>

            <article class="catalog-surface-card">
                <h2><a href="{{ route('game-catalog.creatures.index') }}">{{ __('game_catalog.creatures.title') }}</a></h2>
                <p>{{ __('game_catalog.creatures.intro') }}</p>
                @if ($bestiaryClasses !== [])
                    <ul class="catalog-tag-list" aria-label="{{ __('game_catalog.creatures.bestiary_classes') }}">
                        @foreach (array_slice($bestiaryClasses, 0, 8) as $bestiaryClass)
                            <li><a href="{{ route('game-catalog.creatures.index', ['bestiary_class' => $bestiaryClass]) }}">{{ $bestiaryClass }}</a></li>
                        @endforeach
                    </ul>
                @endif
            </article>
        </div>
    @endif
@endsection
