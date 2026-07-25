@extends('game.layout')

@section('title', 'Events')

@section('content')
    <div class="page-header">
        <p class="eyebrow">Community calendar</p>
        <h1>Events</h1>
        <p class="muted">Approved event times are shown in UTC. Schedules are stored explicitly and never inferred from news text.</p>
    </div>

    @if ($calendar['active'] === [] && $calendar['upcoming'] === [] && $calendar['archived'] === [] && $calendar['cancelled'] === [])
        <div class="empty-state">
            <strong>No events are available.</strong>
            <p>Approved events will appear here when they are scheduled.</p>
        </div>
    @else
        @foreach ([
            'active' => ['Active now', 'Events currently in progress.'],
            'upcoming' => ['Upcoming', 'Approved events that begin later.'],
            'archived' => ['Archived', 'Events whose end boundary has passed.'],
            'cancelled' => ['Cancelled', 'Previously approved events that were cancelled.'],
        ] as $bucket => [$heading, $description])
            @if ($calendar[$bucket] !== [])
                <section aria-labelledby="events-{{ $bucket }}">
                    <div class="section-heading">
                        <p class="eyebrow">{{ ucfirst($bucket) }}</p>
                        <h2 id="events-{{ $bucket }}">{{ $heading }}</h2>
                        <p class="muted">{{ $description }}</p>
                    </div>

                    <div class="card-grid">
                        @foreach ($calendar[$bucket] as $event)
                            <article class="card">
                                <p class="eyebrow">
                                    {{ $event['starts_at']->format('Y-m-d H:i') }}
                                    –
                                    {{ $event['ends_at']->format('Y-m-d H:i') }} UTC
                                </p>
                                <h3><a href="{{ route('events.show', ['slug' => $event['slug']]) }}">{{ $event['title'] }}</a></h3>
                                <p>{{ $event['summary'] }}</p>
                                @if ($event['featured'])
                                    <p><strong>Featured event</strong></p>
                                @endif
                            </article>
                        @endforeach
                    </div>
                </section>
            @endif
        @endforeach
    @endif
@endsection
