<section class="card" aria-labelledby="upcoming-event-title">
    <div class="section-heading">
        <p class="eyebrow">Event calendar</p>
        <h2 id="upcoming-event-title">Next event</h2>
    </div>

    @if ($summary->state === \App\PublicPortal\PublicContentState::AVAILABLE && $summary->event !== null)
        <p class="eyebrow">
            {{ $summary->event['starts_at']->format('Y-m-d H:i') }}
            –
            {{ $summary->event['ends_at']->format('Y-m-d H:i') }} UTC
        </p>
        <h3><a href="{{ route('events.show', ['slug' => $summary->event['slug']]) }}">{{ $summary->event['title'] }}</a></h3>
        <p>{{ $summary->event['summary'] }}</p>
    @elseif ($summary->state === \App\PublicPortal\PublicContentState::EMPTY)
        <div class="empty-state">
            <strong>No scheduled event.</strong>
            <p>Approved upcoming events will appear here.</p>
        </div>
    @else
        <div class="alert alert-danger" role="status">
            Event information is temporarily unavailable.
        </div>
    @endif
</section>
