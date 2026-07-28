@extends('game.layout')

@section('title', __('marketplace.title'))
@section('description', __('marketplace.description'))
@section('page-class', 'bazaar-page')

@section('content')
    <div class="page-header bazaar-hero">
        <div>
            <p class="eyebrow">{{ __('marketplace.browse') }}</p>
            <h1>{{ __('marketplace.title') }}</h1>
            <p class="muted">{{ __('marketplace.description') }}</p>
        </div>
        <div class="action-row">
            @auth
                <a class="button button-secondary" href="{{ route('marketplace.account') }}">{{ __('marketplace.my_bazaar') }}</a>
                <a class="button" href="{{ route('marketplace.listing.create') }}">{{ __('marketplace.sell_character') }}</a>
            @else
                <a class="button" href="{{ route('identity.login.create') }}">{{ __('marketplace.login_to_bid') }}</a>
            @endauth
        </div>
    </div>

    <form class="bazaar-filters" method="GET" action="{{ route('marketplace.index') }}" aria-label="{{ __('marketplace.filters') }}">
        <div class="section-heading">
            <p class="eyebrow">{{ __('marketplace.filters') }}</p>
            <h2>{{ __('marketplace.filters') }}</h2>
        </div>
        <div class="bazaar-filter-grid">
            <label>
                <span>{{ __('marketplace.vocation') }}</span>
                <select name="vocation">
                    <option value="">{{ __('marketplace.all_vocations') }}</option>
                    @foreach ([1, 2, 3, 4, 9] as $vocation)
                        <option value="{{ $vocation }}" @selected(($filters['vocation'] ?? null) === $vocation)>
                            {{ __('marketplace.vocations.'.$vocation) }}
                        </option>
                    @endforeach
                </select>
            </label>
            <label>
                <span>{{ __('marketplace.level_min') }}</span>
                <input type="number" name="level_min" min="1" max="5000" value="{{ $filters['level_min'] ?? '' }}">
            </label>
            <label>
                <span>{{ __('marketplace.level_max') }}</span>
                <input type="number" name="level_max" min="1" max="5000" value="{{ $filters['level_max'] ?? '' }}">
            </label>
            <label>
                <span>{{ __('marketplace.price_min') }}</span>
                <input type="number" name="price_min" min="0" max="1000000000" value="{{ $filters['price_min'] ?? '' }}">
            </label>
            <label>
                <span>{{ __('marketplace.price_max') }}</span>
                <input type="number" name="price_max" min="0" max="1000000000" value="{{ $filters['price_max'] ?? '' }}">
            </label>
            <label>
                <span>{{ __('marketplace.sort') }}</span>
                <select name="sort">
                    @foreach (['ending', 'newest', 'level_desc', 'price_asc', 'price_desc'] as $sort)
                        <option value="{{ $sort }}" @selected(($filters['sort'] ?? 'ending') === $sort)>
                            {{ __('marketplace.sort_'.$sort) }}
                        </option>
                    @endforeach
                </select>
            </label>
        </div>
        <div class="action-row">
            <button class="button" type="submit">{{ __('marketplace.apply_filters') }}</button>
            <a class="button button-secondary" href="{{ route('marketplace.index') }}">{{ __('marketplace.reset_filters') }}</a>
        </div>
    </form>

    @if ($auctions->isEmpty())
        <div class="empty-state bazaar-empty">
            <strong>{{ __('marketplace.empty') }}</strong>
            <p>{{ __('marketplace.empty_help') }}</p>
        </div>
    @else
        <div class="bazaar-grid" data-testid="marketplace-auction-grid">
            @foreach ($auctions as $auction)
                @include('marketplace.partials.auction-card', ['auction' => $auction])
            @endforeach
        </div>

        <div class="bazaar-pagination">
            {{ $auctions->links() }}
        </div>
    @endif
@endsection
