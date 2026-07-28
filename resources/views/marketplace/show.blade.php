@extends('game.layout')

@section('title', $auction->player_name.' · '.__('marketplace.title'))
@section('description', __('marketplace.verified_snapshot_help'))
@section('page-class', 'bazaar-page')

@section('content')
    @php
        $snapshot = $auction->character_snapshot;
        $displayPrice = $auction->current_bid > 0 ? $auction->current_bid : $auction->starting_bid;
        $isActive = $auction->status === \App\Marketplace\Models\CharacterAuction::STATUS_ACTIVE
            && $auction->ends_at !== null
            && $auction->ends_at->isFuture();
        $isSeller = auth()->check() && (int) auth()->id() === $auction->seller_identity_id;
    @endphp

    <nav class="bazaar-breadcrumbs" aria-label="Breadcrumb">
        <a href="{{ route('marketplace.index') }}">{{ __('marketplace.title') }}</a>
        <span aria-hidden="true">/</span>
        <span aria-current="page">{{ $auction->player_name }}</span>
    </nav>

    @if (session('status'))
        <div class="alert alert-success" role="status">{{ session('status') }}</div>
    @endif
    @if ($errors->any())
        <div class="alert alert-danger" role="alert">
            <p><strong>{{ __('marketplace.auction_state') }}</strong></p>
            <ul>
                @foreach ($errors->all() as $error)
                    <li>{{ $error }}</li>
                @endforeach
            </ul>
        </div>
    @endif

    <div class="bazaar-detail-hero">
        <div class="bazaar-detail-portrait" aria-hidden="true">
            <span>{{ mb_strtoupper(mb_substr($auction->player_name, 0, 1)) }}</span>
        </div>
        <div class="bazaar-detail-title">
            <p class="eyebrow">{{ __('marketplace.status_labels.'.$auction->status) }}</p>
            <h1>{{ $auction->player_name }}</h1>
            <p class="muted">
                {{ __('marketplace.level') }} {{ number_format($auction->level, 0, ',', ' ') }} ·
                {{ __('marketplace.vocations.'.$auction->vocation) }}
            </p>
        </div>
        <dl class="bazaar-price-panel">
            <div>
                <dt>{{ $auction->current_bid > 0 ? __('marketplace.current_bid') : __('marketplace.starting_bid') }}</dt>
                <dd>{{ number_format($displayPrice, 0, ',', ' ') }} <small>{{ __('marketplace.coins') }}</small></dd>
            </div>
            <div>
                <dt>{{ __('marketplace.bids') }}</dt>
                <dd>{{ number_format($auction->bid_count, 0, ',', ' ') }}</dd>
            </div>
            @if ($auction->ends_at !== null)
                <div>
                    <dt>{{ __('marketplace.ends') }}</dt>
                    <dd><time datetime="{{ $auction->ends_at->toAtomString() }}">{{ $auction->ends_at->utc()->format('Y-m-d H:i') }} UTC</time></dd>
                </div>
            @endif
        </dl>
    </div>

    <div class="bazaar-detail-layout">
        <div class="bazaar-detail-main">
            <section class="bazaar-panel" aria-labelledby="snapshot-heading">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('marketplace.verified_snapshot') }}</p>
                    <h2 id="snapshot-heading">{{ __('marketplace.verified_snapshot') }}</h2>
                    <p class="muted">{{ __('marketplace.verified_snapshot_help') }}</p>
                </div>
                <dl class="bazaar-stat-grid">
                    <div><dt>{{ __('marketplace.level') }}</dt><dd>{{ number_format((int) ($snapshot['level'] ?? $auction->level), 0, ',', ' ') }}</dd></div>
                    <div><dt>{{ __('marketplace.vocation') }}</dt><dd>{{ __('marketplace.vocations.'.($snapshot['vocation'] ?? $auction->vocation)) }}</dd></div>
                    <div><dt>Experience</dt><dd>{{ number_format((int) ($snapshot['experience'] ?? 0), 0, ',', ' ') }}</dd></div>
                    <div><dt>Town ID</dt><dd>{{ number_format((int) ($snapshot['town_id'] ?? 0), 0, ',', ' ') }}</dd></div>
                    <div><dt>Last login</dt><dd>{{ (int) ($snapshot['lastlogin'] ?? 0) > 0 ? gmdate('Y-m-d H:i', (int) $snapshot['lastlogin']).' UTC' : '—' }}</dd></div>
                    <div><dt>Last logout</dt><dd>{{ (int) ($snapshot['lastlogout'] ?? 0) > 0 ? gmdate('Y-m-d H:i', (int) $snapshot['lastlogout']).' UTC' : '—' }}</dd></div>
                </dl>
            </section>

            <section class="bazaar-panel" aria-labelledby="skills-heading">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('marketplace.skills') }}</p>
                    <h2 id="skills-heading">{{ __('marketplace.skills') }}</h2>
                </div>
                <dl class="bazaar-skill-grid">
                    @foreach (['maglevel', 'skill_fist', 'skill_club', 'skill_sword', 'skill_axe', 'skill_dist', 'skill_shielding', 'skill_fishing'] as $skill)
                        <div>
                            <dt>{{ __('marketplace.skill_labels.'.$skill) }}</dt>
                            <dd>{{ number_format((int) ($snapshot[$skill] ?? 0), 0, ',', ' ') }}</dd>
                        </div>
                    @endforeach
                </dl>
            </section>

            <section class="bazaar-panel" aria-labelledby="appearance-heading">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('marketplace.appearance') }}</p>
                    <h2 id="appearance-heading">{{ __('marketplace.appearance') }}</h2>
                </div>
                <dl class="bazaar-stat-grid">
                    @foreach (['looktype', 'lookaddons', 'lookhead', 'lookbody', 'looklegs', 'lookfeet'] as $appearance)
                        <div><dt>{{ str_replace('look', 'Look ', ucfirst($appearance)) }}</dt><dd>{{ number_format((int) ($snapshot[$appearance] ?? 0), 0, ',', ' ') }}</dd></div>
                    @endforeach
                </dl>
            </section>

            <section class="bazaar-panel" aria-labelledby="history-heading">
                <div class="section-heading">
                    <p class="eyebrow">{{ __('marketplace.bid_history') }}</p>
                    <h2 id="history-heading">{{ __('marketplace.bid_history') }}</h2>
                </div>
                @if ($bids->isEmpty())
                    <p class="muted">{{ __('marketplace.bid_history_empty') }}</p>
                @else
                    <ol class="bazaar-bid-history">
                        @foreach ($bids as $bid)
                            <li>
                                <span>{{ __('marketplace.anonymous_bidder') }}</span>
                                <strong>{{ number_format($bid->amount, 0, ',', ' ') }} {{ __('marketplace.coins') }}</strong>
                                <time datetime="{{ $bid->placed_at->toAtomString() }}">{{ $bid->placed_at->utc()->format('Y-m-d H:i:s') }} UTC</time>
                            </li>
                        @endforeach
                    </ol>
                @endif
            </section>
        </div>

        <aside class="bazaar-action-panel" aria-label="{{ __('marketplace.actions') }}">
            @auth
                <form method="POST" action="{{ $isWatched ? route('marketplace.watch.destroy', $auction) : route('marketplace.watch.store', $auction) }}">
                    @csrf
                    @if ($isWatched)
                        @method('DELETE')
                    @endif
                    <button class="button button-secondary button-block" type="submit">
                        {{ $isWatched ? __('marketplace.unwatch') : __('marketplace.watch') }}
                    </button>
                </form>

                @if ($isActive && ! $isSeller)
                    <form class="bazaar-bid-form" method="POST" action="{{ route('marketplace.bids.store', $auction) }}">
                        @csrf
                        <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                        <label for="bid-amount">
                            <span>{{ __('marketplace.amount') }}</span>
                            <input id="bid-amount" type="number" name="amount" min="{{ $auction->minimumNextBid() }}" max="1000000000" required value="{{ old('amount', $auction->minimumNextBid()) }}">
                        </label>
                        <p class="muted">{{ __('marketplace.minimum_next_bid', ['amount' => number_format($auction->minimumNextBid(), 0, ',', ' ')]) }}</p>
                        <button class="button button-block" type="submit">{{ __('marketplace.confirm_bid') }}</button>
                    </form>

                    @if ($auction->buy_now_price !== null)
                        <form method="POST" action="{{ route('marketplace.purchase', $auction) }}">
                            @csrf
                            <input type="hidden" name="request_id" value="{{ \Illuminate\Support\Str::uuid() }}">
                            <input type="hidden" name="amount" value="{{ $auction->buy_now_price }}">
                            <button class="button button-accent button-block" type="submit">
                                {{ __('marketplace.confirm_buy_now', ['amount' => number_format($auction->buy_now_price, 0, ',', ' ')]) }}
                            </button>
                        </form>
                    @endif
                @elseif ($isSeller)
                    <p class="muted">{{ __('marketplace.cannot_cancel_with_bids') }}</p>
                @endif

                <a class="button button-secondary button-block" href="{{ route('marketplace.account') }}">{{ __('marketplace.my_bazaar') }}</a>
            @else
                <a class="button button-block" href="{{ route('identity.login.create') }}">{{ __('marketplace.login_to_bid') }}</a>
            @endauth
        </aside>
    </div>
@endsection
