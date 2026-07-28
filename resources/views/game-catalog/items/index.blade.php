@extends('game.layout')

@section('title', __('game_catalog.items.title'))
@section('description', __('game_catalog.items.intro'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow"><a href="{{ route('game-catalog.index') }}">{{ __('game_catalog.title') }}</a></p>
        <h1>{{ __('game_catalog.items.title') }}</h1>
        <p class="muted">{{ __('game_catalog.items.intro') }}</p>
    </header>

    @include('game-catalog.partials.context')

    @if ($context !== null)
        <form class="catalog-filters" method="get" action="{{ route('game-catalog.items.index') }}" aria-label="{{ __('game_catalog.filters.label') }}">
            <label>
                <span>{{ __('game_catalog.filters.search') }}</span>
                <input type="search" name="q" maxlength="100" value="{{ $filters['q'] }}">
            </label>
            <label>
                <span>{{ __('game_catalog.items.category') }}</span>
                <select name="category">
                    <option value="">{{ __('game_catalog.filters.all') }}</option>
                    @foreach ($categories as $category)
                        <option value="{{ $category->category }}" @selected($filters['category'] === $category->category)>{{ $category->category }} ({{ $category->item_count }})</option>
                    @endforeach
                </select>
            </label>
            <label>
                <span>{{ __('game_catalog.items.weapon_type') }}</span>
                <input type="text" name="weapon_type" maxlength="40" pattern="[a-z0-9][a-z0-9._-]*" value="{{ $filters['weapon_type'] }}">
            </label>
            <button type="submit">{{ __('game_catalog.filters.apply') }}</button>
            <a class="button-link" href="{{ route('game-catalog.items.index') }}">{{ __('game_catalog.filters.clear') }}</a>
        </form>

        @if ($items->isEmpty())
            <div class="empty-state" role="status">
                <strong>{{ __('game_catalog.items.empty') }}</strong>
                <p>{{ __('game_catalog.items.empty_help') }}</p>
            </div>
        @else
            <p class="catalog-result-count">{{ trans_choice('game_catalog.items.result_count', $items->total(), ['count' => $items->total()]) }}</p>
            <div class="catalog-card-grid">
                @foreach ($items as $item)
                    <article class="catalog-card">
                        <div class="catalog-image-placeholder" aria-hidden="true">
                            @if ($item->image_key !== null)
                                <span>{{ $item->image_key }}</span>
                            @else
                                <span>?</span>
                            @endif
                        </div>
                        <div>
                            <p class="catalog-kicker">{{ $item->category }}@if($item->weapon_type !== null) · {{ $item->weapon_type }}@endif</p>
                            <h2><a href="{{ route('game-catalog.items.show', ['slug' => \Illuminate\Support\Str::after($item->canonical_key, ':')]) }}">{{ $item->name }}</a></h2>
                            <dl class="catalog-compact-stats">
                                <div><dt>{{ __('game_catalog.items.attack') }}</dt><dd>{{ $item->attack ?? __('game_catalog.unknown') }}</dd></div>
                                <div><dt>{{ __('game_catalog.items.defense') }}</dt><dd>{{ $item->defense ?? __('game_catalog.unknown') }}</dd></div>
                                <div><dt>{{ __('game_catalog.items.level') }}</dt><dd>{{ $item->minimum_level ?? __('game_catalog.unknown') }}</dd></div>
                                <div><dt>{{ __('game_catalog.items.availability') }}</dt><dd>{{ $item->availability }}</dd></div>
                            </dl>
                        </div>
                    </article>
                @endforeach
            </div>
            <div class="catalog-pagination">{{ $items->links() }}</div>
        @endif
    @endif
@endsection
