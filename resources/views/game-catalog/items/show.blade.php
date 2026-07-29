@extends('game.layout')

@section('title', $item->name)
@section('description', $item->description ?? $item->name)
@section('page-class', 'game-catalog-page catalog-detail')

@push('styles')
    <link rel="stylesheet" href="{{ asset('css/game-catalog.css') }}">
@endpush

@section('content')
    <header class="page-header catalog-hero">
        <p class="eyebrow"><a href="{{ route('game-catalog.items.index') }}">{{ __('game_catalog.back_to_items') }}</a></p>
        <h1>{{ $item->name }}</h1>
        <div class="catalog-meta">
            <span class="catalog-chip">{{ $item->category }}</span>
            @if ($item->weaponType !== null)<span class="catalog-chip">{{ $item->weaponType }}</span>@endif
        </div>
        @if ($item->description !== null && $item->description !== '')<p>{{ $item->description }}</p>@endif
    </header>

    <section aria-labelledby="item-attributes">
        <h2 id="item-attributes">{{ __('game_catalog.items') }}</h2>
        <dl class="catalog-detail-list">
            <div><dt>{{ __('game_catalog.server_id') }}</dt><dd>{{ $item->serverId }}</dd></div>
            @if ($item->clientId !== null)<div><dt>{{ __('game_catalog.client_id') }}</dt><dd>{{ $item->clientId }}</dd></div>@endif
            @if ($item->attack !== null)<div><dt>{{ __('game_catalog.attack') }}</dt><dd>{{ $item->attack }}</dd></div>@endif
            @if ($item->defense !== null)<div><dt>{{ __('game_catalog.defense') }}</dt><dd>{{ $item->defense }}</dd></div>@endif
            @if ($item->extraDefense !== null)<div><dt>{{ __('game_catalog.extra_defense') }}</dt><dd>{{ $item->extraDefense }}</dd></div>@endif
            @if ($item->armor !== null)<div><dt>{{ __('game_catalog.armor') }}</dt><dd>{{ $item->armor }}</dd></div>@endif
            @if ($item->range !== null)<div><dt>{{ __('game_catalog.range') }}</dt><dd>{{ $item->range }}</dd></div>@endif
            @if ($item->weight !== null)<div><dt>{{ __('game_catalog.weight') }}</dt><dd>{{ $item->weight }}</dd></div>@endif
            @if ($item->minimumLevel !== null)<div><dt>{{ __('game_catalog.level') }}</dt><dd>{{ $item->minimumLevel }}</dd></div>@endif
            @if ($item->imbuementSlots !== null)<div><dt>{{ __('game_catalog.imbuement_slots') }}</dt><dd>{{ $item->imbuementSlots }}</dd></div>@endif
            @if ($item->elementType !== null)<div><dt>{{ __('game_catalog.element') }}</dt><dd>{{ $item->elementType }}@if ($item->elementValue !== null) {{ $item->elementValue }}@endif</dd></div>@endif
            <div><dt>{{ __('game_catalog.stackable') }}</dt><dd>{{ $item->stackable ? __('game_catalog.yes') : __('game_catalog.no') }}</dd></div>
            <div><dt>{{ __('game_catalog.pickupable') }}</dt><dd>{{ $item->pickupable ? __('game_catalog.yes') : __('game_catalog.no') }}</dd></div>
            @if ($item->vocations !== [])<div><dt>{{ __('game_catalog.vocations') }}</dt><dd>{{ implode(', ', $item->vocations) }}</dd></div>@endif
        </dl>
    </section>

    <section aria-labelledby="item-sources">
        <h2 id="item-sources">{{ __('game_catalog.sources') }}</h2>
        @if ($item->sources === [])
            <div class="empty-state" role="status"><strong>{{ __('game_catalog.no_sources') }}</strong></div>
        @else
            <ul class="catalog-relation-list">
                @foreach ($item->sources as $source)
                    <li>
                        <a href="{{ route('game-catalog.creatures.show', ['slug' => $source->slug]) }}">{{ $source->name }}</a>
                        @if ($source->chanceModel === 'rational_probability')
                            <span>{{ __('game_catalog.chance') }}: {{ __('game_catalog.chance_ratio', ['numerator' => $source->chanceNumerator, 'denominator' => $source->chanceDenominator]) }}</span>
                        @else
                            <span>{{ __('game_catalog.chance') }}: {{ __('game_catalog.chance_threshold', ['threshold' => $source->chanceThreshold, 'maximum' => $source->rollMaximum]) }}</span>
                        @endif
                        <span>{{ __('game_catalog.count') }}: {{ $source->minimumCount === $source->maximumCount ? __('game_catalog.count_single', ['count' => $source->minimumCount]) : __('game_catalog.count_range', ['minimum' => $source->minimumCount, 'maximum' => $source->maximumCount]) }}</span>
                    </li>
                @endforeach
            </ul>
        @endif
    </section>
@endsection
