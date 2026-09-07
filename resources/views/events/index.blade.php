@extends('game.layout')

@section('title', __('public.events.title'))

@section('portal-family', 'events')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.events.calendar') }}</p>
        <h1>{{ __('public.events.title') }}</h1>
        <p class="muted">{{ __('public.events.description') }}</p>
    </div>

    @if ($calendar['active'] === [] && $calendar['upcoming'] === [] && $calendar['archived'] === [] && $calendar['cancelled'] === [])
        <div class="empty-state">
            <strong>{{ __('public.events.empty') }}</strong>
            <p>{{ __('public.events.empty_help') }}</p>
        </div>
    @else
        @foreach ([
            'active' => [__('public.events.active'), __('public.events.active_help')],
            'upcoming' => [__('public.events.upcoming'), __('public.events.upcoming_help')],
            'archived' => [__('public.events.archived'), __('public.events.archived_help')],
            'cancelled' => [__('public.events.cancelled'), __('public.events.cancelled_help')],
        ] as $bucket => [$heading, $description])
            @if ($calendar[$bucket] !== [])
                <section class="calendar-bucket calendar-bucket-{{ $bucket }}" aria-labelledby="events-{{ $bucket }}">
                    <div class="section-heading">
                        <p class="eyebrow">{{ $heading }}</p>
                        <h2 id="events-{{ $bucket }}">{{ $heading }}</h2>
                        <p class="muted">{{ $description }}</p>
                    </div>

                    <div class="event-timeline">
                        @foreach ($calendar[$bucket] as $event)
                            <article class="card event-timeline-entry">
                                <time class="event-date" datetime="{{ $event['starts_at']->toAtomString() }}">{{ $localeFormatter->date($event['starts_at']) }}</time>
                                <div class="event-timeline-copy">
                                <p class="eyebrow">
                                    {{ $localeFormatter->dateTime($event['starts_at']) }}
                                    –
                                    {{ $localeFormatter->dateTime($event['ends_at']) }} UTC
                                </p>
                                <h3><a href="{{ route('events.show', ['slug' => $event['slug']]) }}">{{ $event['title'] }}</a></h3>
                                <p>{{ $event['summary'] }}</p>
                                @if ($event['featured'])
                                    <p><strong>{{ __('public.events.featured') }}</strong></p>
                                @endif
                                </div>
                            </article>
                        @endforeach
                    </div>
                </section>
            @endif
        @endforeach
    @endif
@endsection
