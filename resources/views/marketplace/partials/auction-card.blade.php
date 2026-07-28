@php
    $displayPrice = $auction->current_bid > 0 ? $auction->current_bid : $auction->starting_bid;
    $statusLabel = __('marketplace.status_labels.'.$auction->status);
    $vocationLabel = __('marketplace.vocations.'.$auction->vocation);
@endphp

<article class="bazaar-card">
    <div class="bazaar-card__portrait" aria-hidden="true">
        <span>{{ mb_strtoupper(mb_substr($auction->player_name, 0, 1)) }}</span>
    </div>
    <div class="bazaar-card__body">
        <div class="bazaar-card__heading">
            <div>
                <p class="eyebrow">{{ $statusLabel }}</p>
                <h2><a href="{{ route('marketplace.show', $auction) }}">{{ $auction->player_name }}</a></h2>
            </div>
            <span class="bazaar-level">{{ __('marketplace.level') }} {{ number_format($auction->level, 0, ',', ' ') }}</span>
        </div>

        <dl class="bazaar-card__facts">
            <div>
                <dt>{{ __('marketplace.vocation') }}</dt>
                <dd>{{ $vocationLabel }}</dd>
            </div>
            <div>
                <dt>{{ $auction->current_bid > 0 ? __('marketplace.current_bid') : __('marketplace.starting_bid') }}</dt>
                <dd>{{ number_format($displayPrice, 0, ',', ' ') }} {{ __('marketplace.coins') }}</dd>
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

        <div class="bazaar-card__footer">
            @if ($auction->buy_now_price !== null)
                <span class="bazaar-buy-now">{{ __('marketplace.buy_now') }}: {{ number_format($auction->buy_now_price, 0, ',', ' ') }}</span>
            @endif
            <a class="button" href="{{ route('marketplace.show', $auction) }}">{{ __('marketplace.details') }}</a>
        </div>
    </div>
</article>
