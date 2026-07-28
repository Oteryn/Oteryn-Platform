@extends('game.layout')

@section('title', __('game_catalog.items'))
@section('description', __('game_catalog.search_placeholder_items'))
@section('page-class', 'game-catalog-page')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow"><a href="{{ route('game-catalog.index') }}">{{ __('game_catalog.title') }}</a></p>
        <h1>{{ __('game_catalog.items') }}</h1>
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
        <form class="catalog-filters" method="get" action="{{ route('game-catalog.items.index') }}">
            <label class="catalog-field">
                <span>{{ __('game_catalog.search') }}</span>
                <input type="search" name="q" value="{{ $catalog->query }}" maxlength="80" placeholder="{{ __('game_catalog.search_placeholder_items') }}">
            </label>
            <label class="catalog-field">
                <span>{{ __('game_catalog.category') }}</span>
                <select name="category">
                    <option value="">{{ __('game_catalog.all_categories') }}</option>
                    @foreach ($catalog->categories as $category)
                        <option value="{{ $category }}" @selected($catalog->category === $category)>{{ $category }}</option>
                    @endforeach
                </select>
            </label>
            <label class="catalog-field">
                <span>{{ __('game_catalog.weapon_type') }}</span>
                <select name="weapon_type">
                    <option value="">{{ __('game_catalog.all_weapon_types') }}</option>
                    @foreach ($catalog->weaponTypes as $weaponType)
                        <option value="{{ $weaponType }}" @selected($catalog->weaponType === $weaponType)>{{ $weaponType }}</option>
                    @endforeach
                </select>
            </label>
            <div>
                <button type="submit">{{ __('game_catalog.filter') }}</button>
                <a href="{{ route('game-catalog.items.index') }}">{{ __('game_catalog.clear_filters') }}</a>
            </div>
        </form>

        @if ($catalog->items === [])
            <div class="empty-state" role="status">
                <strong>{{ __('game_catalog.empty_items') }}</strong>
            </div>
        @else
            <section class="catalog-grid" aria-label="{{ __('game_catalog.items') }}">
                @foreach ($catalog->items as $item)
                    <article class="card catalog-card">
                        <p class="eyebrow">{{ $item->category }}@if ($item->weaponType !== null) · {{ $item->weaponType }}@endif</p>
                        <h2><a href="{{ route('game-catalog.items.show', ['slug' => $item->slug]) }}">{{ $item->name }}</a></h2>
                        @if ($item->summary !== null && $item->summary !== '')
                            <p>{{ $item->summary }}</p>
                        @endif
                        <dl class="catalog-stats">
                            @if ($item->attack !== null)<div><dt>{{ __('game_catalog.attack') }}</dt><dd>{{ $item->attack }}</dd></div>@endif
                            @if ($item->defense !== null)<div><dt>{{ __('game_catalog.defense') }}</dt><dd>{{ $item->defense }}</dd></div>@endif
                            @if ($item->armor !== null)<div><dt>{{ __('game_catalog.armor') }}</dt><dd>{{ $item->armor }}</dd></div>@endif
                            @if ($item->minimumLevel !== null)<div><dt>{{ __('game_catalog.level') }}</dt><dd>{{ $item->minimumLevel }}</dd></div>@endif
                        </dl>
                    </article>
                @endforeach
            </section>
        @endif

        @if ($catalog->lastPage() > 1)
            <nav class="catalog-pagination" aria-label="{{ __('game_catalog.page', ['current' => $catalog->page, 'last' => $catalog->lastPage()]) }}">
                @if ($catalog->page > 1)
                    <a class="button" href="{{ route('game-catalog.items.index', array_merge(request()->only(['q', 'category', 'weapon_type']), ['page' => $catalog->page - 1])) }}">{{ __('game_catalog.previous') }}</a>
                @else
                    <span></span>
                @endif
                <span>{{ __('game_catalog.page', ['current' => $catalog->page, 'last' => $catalog->lastPage()]) }}</span>
                @if ($catalog->page < $catalog->lastPage())
                    <a class="button" href="{{ route('game-catalog.items.index', array_merge(request()->only(['q', 'category', 'weapon_type']), ['page' => $catalog->page + 1])) }}">{{ __('game_catalog.next') }}</a>
                @endif
            </nav>
        @endif
    @endif
@endsection
