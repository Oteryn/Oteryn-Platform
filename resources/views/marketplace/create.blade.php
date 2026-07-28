@extends('game.layout')

@section('title', __('marketplace.create_title'))
@section('robots', 'noindex,nofollow')
@section('page-class', 'bazaar-page')

@section('content')
    <nav class="bazaar-breadcrumbs" aria-label="Breadcrumb">
        <a href="{{ route('marketplace.account') }}">{{ __('marketplace.my_bazaar') }}</a>
        <span aria-hidden="true">/</span>
        <span aria-current="page">{{ __('marketplace.create_title') }}</span>
    </nav>

    <div class="page-header">
        <p class="eyebrow">{{ __('marketplace.sell_character') }}</p>
        <h1>{{ __('marketplace.create_title') }}</h1>
        <p class="muted">{{ __('marketplace.create_description') }}</p>
    </div>

    @if ($errors->any())
        <div class="alert alert-danger" role="alert">
            <ul>
                @foreach ($errors->all() as $error)
                    <li>{{ $error }}</li>
                @endforeach
            </ul>
        </div>
    @endif

    @if (! $bindingReady)
        <div class="empty-state">
            <strong>{{ __('marketplace.binding_not_ready') }}</strong>
            <p><a class="button" href="{{ route('account.overview') }}">{{ __('marketplace.my_bazaar') }}</a></p>
        </div>
    @elseif (! $charactersAvailable)
        <div class="empty-state">
            <strong>{{ __('marketplace.characters_unavailable') }}</strong>
        </div>
    @elseif ($characters->isEmpty())
        <div class="empty-state">
            <strong>{{ __('marketplace.no_characters') }}</strong>
        </div>
    @else
        <div class="bazaar-create-layout">
            <form class="bazaar-listing-form" method="POST" action="{{ route('marketplace.listing.store') }}">
                @csrf
                <input type="hidden" name="request_id" value="{{ old('request_id', \Illuminate\Support\Str::uuid()) }}">

                <label for="player-id">
                    <span>{{ __('marketplace.character') }}</span>
                    <select id="player-id" name="player_id" required>
                        @foreach ($characters as $character)
                            <option value="{{ $character->id }}" @selected((int) old('player_id') === (int) $character->id)>
                                {{ $character->name }} · {{ __('marketplace.level') }} {{ $character->level }} · {{ __('marketplace.vocations.'.$character->vocation) }}
                            </option>
                        @endforeach
                    </select>
                </label>

                <label for="duration-days">
                    <span>{{ __('marketplace.duration') }}</span>
                    <select id="duration-days" name="duration_days" required>
                        @foreach ($durations as $duration)
                            <option value="{{ $duration }}" @selected((int) old('duration_days', 3) === (int) $duration)>
                                {{ trans_choice('marketplace.duration_days', $duration, ['count' => $duration]) }}
                            </option>
                        @endforeach
                    </select>
                </label>

                <label for="starting-bid">
                    <span>{{ __('marketplace.starting_bid') }}</span>
                    <input id="starting-bid" type="number" name="starting_bid" min="{{ config('marketplace.minimum_starting_bid') }}" max="1000000000" required value="{{ old('starting_bid', config('marketplace.minimum_starting_bid')) }}">
                </label>

                <label for="buy-now-price">
                    <span>{{ __('marketplace.optional_buy_now') }}</span>
                    <input id="buy-now-price" type="number" name="buy_now_price" min="{{ config('marketplace.minimum_starting_bid') }}" max="1000000000" value="{{ old('buy_now_price') }}">
                </label>

                <button class="button" type="submit">{{ __('marketplace.submit_listing') }}</button>
            </form>

            <aside class="bazaar-panel bazaar-listing-summary">
                <p class="eyebrow">{{ __('marketplace.request_summary') }}</p>
                <h2>{{ __('marketplace.request_summary') }}</h2>
                <p>{{ __('marketplace.request_summary_help') }}</p>
                <ul>
                    <li>{{ __('marketplace.create_description') }}</li>
                    <li>{{ __('marketplace.verified_snapshot_help') }}</li>
                    <li>{{ __('marketplace.cannot_cancel_with_bids') }}</li>
                </ul>
            </aside>
        </div>
    @endif
@endsection
