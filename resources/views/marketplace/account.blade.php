@extends('game.layout')

@section('title', __('marketplace.my_bazaar'))
@section('robots', 'noindex,nofollow')
@section('page-class', 'bazaar-page')

@section('content')
    <div class="page-header bazaar-hero">
        <div>
            <p class="eyebrow">{{ __('marketplace.my_bazaar') }}</p>
            <h1>{{ __('marketplace.my_bazaar') }}</h1>
            <p class="muted">{{ __('marketplace.description') }}</p>
        </div>
        <div class="action-row">
            <a class="button button-secondary" href="{{ route('marketplace.index') }}">{{ __('marketplace.browse') }}</a>
            <a class="button" href="{{ route('marketplace.listing.create') }}">{{ __('marketplace.sell_character') }}</a>
        </div>
    </div>

    @if (session('status'))
        <div class="alert alert-success" role="status">{{ session('status') }}</div>
    @endif
    @if ($errors->any())
        <div class="alert alert-danger" role="alert">
            <ul>
                @foreach ($errors->all() as $error)
                    <li>{{ $error }}</li>
                @endforeach
            </ul>
        </div>
    @endif

    <section class="bazaar-wallet" aria-labelledby="wallet-heading">
        <div>
            <p class="eyebrow">{{ __('marketplace.wallet') }}</p>
            <h2 id="wallet-heading">{{ __('marketplace.wallet') }}</h2>
        </div>
        <dl>
            <div>
                <dt>{{ __('marketplace.available_balance') }}</dt>
                <dd>{{ number_format($wallet->available_balance, 0, ',', ' ') }} <small>{{ __('marketplace.coins') }}</small></dd>
            </div>
            <div>
                <dt>{{ __('marketplace.reserved_balance') }}</dt>
                <dd>{{ number_format($wallet->reserved_balance, 0, ',', ' ') }} <small>{{ __('marketplace.coins') }}</small></dd>
            </div>
        </dl>
    </section>

    @foreach ([
        'selling' => __('marketplace.my_auctions'),
        'bids' => __('marketplace.my_bids'),
        'watched' => __('marketplace.watched'),
        'history' => __('marketplace.history'),
    ] as $collectionName => $heading)
        <section class="bazaar-account-section" aria-labelledby="section-{{ $collectionName }}">
            <div class="section-heading">
                <p class="eyebrow">{{ $heading }}</p>
                <h2 id="section-{{ $collectionName }}">{{ $heading }}</h2>
            </div>

            @if ($$collectionName->isEmpty())
                <div class="empty-state">
                    <p>{{ __('marketplace.section_empty') }}</p>
                </div>
            @else
                <div class="table-wrap bazaar-account-table">
                    <table>
                        <thead>
                        <tr>
                            <th>{{ __('marketplace.character') }}</th>
                            <th>{{ __('marketplace.status') }}</th>
                            <th>{{ __('marketplace.price') }}</th>
                            <th>{{ __('marketplace.ends') }}</th>
                            <th>{{ __('marketplace.actions') }}</th>
                        </tr>
                        </thead>
                        <tbody>
                        @foreach ($$collectionName as $auction)
                            <tr>
                                <td data-label="{{ __('marketplace.character') }}">
                                    <a href="{{ route('marketplace.show', $auction) }}"><strong>{{ $auction->player_name }}</strong></a><br>
                                    <span class="muted">{{ __('marketplace.level') }} {{ $auction->level }} · {{ __('marketplace.vocations.'.$auction->vocation) }}</span>
                                </td>
                                <td data-label="{{ __('marketplace.status') }}">{{ __('marketplace.status_labels.'.$auction->status) }}</td>
                                <td data-label="{{ __('marketplace.price') }}">{{ number_format($auction->current_bid > 0 ? $auction->current_bid : $auction->starting_bid, 0, ',', ' ') }} {{ __('marketplace.coins') }}</td>
                                <td data-label="{{ __('marketplace.ends') }}">
                                    @if ($auction->ends_at !== null)
                                        <time datetime="{{ $auction->ends_at->toAtomString() }}">{{ $auction->ends_at->utc()->format('Y-m-d H:i') }} UTC</time>
                                    @else
                                        —
                                    @endif
                                </td>
                                <td data-label="{{ __('marketplace.actions') }}">
                                    <div class="action-row">
                                        <a class="button button-secondary" href="{{ route('marketplace.show', $auction) }}">{{ __('marketplace.details') }}</a>
                                        @if ($collectionName === 'selling' && in_array($auction->status, [\App\Marketplace\Models\CharacterAuction::STATUS_ESCROW_PENDING, \App\Marketplace\Models\CharacterAuction::STATUS_ACTIVE], true) && $auction->bid_count === 0)
                                            <form method="POST" action="{{ route('marketplace.listing.cancel', $auction) }}">
                                                @csrf
                                                <button class="button button-danger" type="submit">{{ __('marketplace.cancel') }}</button>
                                            </form>
                                        @endif
                                        @if ($collectionName === 'watched')
                                            <form method="POST" action="{{ route('marketplace.watch.destroy', $auction) }}">
                                                @csrf
                                                @method('DELETE')
                                                <button class="button button-secondary" type="submit">{{ __('marketplace.unwatch') }}</button>
                                            </form>
                                        @endif
                                    </div>
                                </td>
                            </tr>
                        @endforeach
                        </tbody>
                    </table>
                </div>
            @endif
        </section>
    @endforeach
@endsection
