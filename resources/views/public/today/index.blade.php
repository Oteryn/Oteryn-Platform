@extends('game.layout')

@section('title', __('today.title'))
@section('description', __('today.description'))
@section('page-class', 'today-page')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header" data-today-state="{{ $today->state->value }}">
        <p class="eyebrow">{{ __('today.eyebrow') }}</p>
        <h1>{{ __('today.title') }}</h1>
        <p class="muted">{{ __('today.description') }}</p>
        @if ($today->state === \App\PublicPortal\Today\TodayPageState::PARTIAL)
            <p class="alert" role="status">{{ __('today.partial') }}</p>
        @elseif ($today->state === \App\PublicPortal\Today\TodayPageState::UNAVAILABLE)
            <p class="alert alert-danger" role="status">{{ __('today.unavailable') }}</p>
        @endif
    </div>

    <div class="card-grid" aria-label="{{ __('today.cards_label') }}">
        @foreach ($today->cards as $card)
            <section
                id="today-{{ $card->kind }}"
                class="card"
                data-today-card="{{ $card->kind }}"
                data-content-state="{{ $card->state->value }}"
                data-source-owner="{{ $card->sourceOwner }}"
                data-source-identity="{{ $card->sourceIdentity }}"
                data-schema-version="{{ $card->schemaVersion }}"
                @if ($card->kind === 'liveops') data-today-runtime-evidence="absent" @endif
                aria-labelledby="today-{{ $card->kind }}-title"
            >
                <div class="section-heading">
                    <p class="eyebrow">{{ __('today.cards.'.$card->kind.'.eyebrow') }}</p>
                    <h2 id="today-{{ $card->kind }}-title">{{ __('today.cards.'.$card->kind.'.title') }}</h2>
                </div>

                @if ($card->state === \App\PublicPortal\Today\TodayCardState::UNAVAILABLE)
                    <div class="alert alert-danger" role="status">
                        {{ __('today.cards.'.$card->kind.'.unavailable') }}
                    </div>
                @elseif ($card->state === \App\PublicPortal\Today\TodayCardState::EMPTY)
                    <div class="empty-state">
                        <strong>{{ __('today.cards.'.$card->kind.'.empty') }}</strong>
                        <p>{{ __('today.cards.'.$card->kind.'.empty_help') }}</p>
                    </div>
                @else
                    <div class="stack">
                        @foreach ($card->items as $item)
                            <article data-today-item="{{ $item->publicId }}">
                                @if ($item->badge !== null)
                                    <p class="eyebrow">{{ $item->badge }}</p>
                                @endif
                                <h3>
                                    @if ($item->url !== null)
                                        <a href="{{ $item->url }}">{{ $item->title }}</a>
                                    @else
                                        {{ $item->title }}
                                    @endif
                                </h3>
                                @if ($item->effectiveAt !== null)
                                    <p class="muted">{{ $localeFormatter->dateTime($item->effectiveAt) }} UTC</p>
                                @endif
                                @if ($item->summary !== null)
                                    <p>{{ $item->summary }}</p>
                                @endif
                                @if ($item->url !== null && $item->actionLabel !== null)
                                    <a href="{{ $item->url }}" @if (str_starts_with($item->url, 'https://')) rel="noopener noreferrer" @endif>
                                        {{ $item->actionLabel }}
                                    </a>
                                @endif
                            </article>
                        @endforeach
                    </div>
                @endif

                @if ($card->canonicalSourceUrl !== null)
                    <p><a href="{{ $card->canonicalSourceUrl }}">{{ __('today.source') }}</a></p>
                @endif
            </section>
        @endforeach
    </div>
@endsection
