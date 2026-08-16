<section class="card" data-content-state="{{ $summary->state->value }}" aria-labelledby="upcoming-event-title">
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="section-heading">
        <p class="eyebrow">{{ __('public.events.calendar') }}</p>
        <h2 id="upcoming-event-title">{{ __('public.events.next_event') }}</h2>
    </div>

    @if ($summary->state === \App\Events\ViewModels\UpcomingEventState::AVAILABLE && $summary->event !== null)
        <p class="eyebrow">
            {{ $localeFormatter->dateTime($summary->event['starts_at']) }}
            –
            {{ $localeFormatter->dateTime($summary->event['ends_at']) }} UTC
        </p>
        <h3><a href="{{ route('events.show', ['slug' => $summary->event['slug']]) }}">{{ $summary->event['title'] }}</a></h3>
        <p>{{ $summary->event['summary'] }}</p>
    @elseif ($summary->state === \App\Events\ViewModels\UpcomingEventState::EMPTY)
        <div class="empty-state">
            <strong>{{ __('public.events.no_scheduled') }}</strong>
            <p>{{ __('public.events.no_scheduled_help') }}</p>
        </div>
    @else
        <div class="alert alert-danger" role="status">
            {{ __('public.events.unavailable') }}
        </div>
    @endif
</section>
