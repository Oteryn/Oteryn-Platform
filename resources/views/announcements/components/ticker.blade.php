<section class="card" data-content-state="{{ $ticker->state->value }}" aria-labelledby="announcement-ticker-title">
    <div class="section-heading">
        <p class="eyebrow">{{ __('public.announcements.notices') }}</p>
        <h2 id="announcement-ticker-title">{{ __('public.announcements.title') }}</h2>
    </div>

    @if ($ticker->state === \App\Announcements\ViewModels\AnnouncementTickerState::AVAILABLE)
        <div class="stack">
            @foreach ($ticker->items as $announcement)
                <article class="notice notice-{{ $announcement->severity }}">
                    <p class="eyebrow">{{ ucfirst($announcement->severity) }}</p>
                    <h3>{{ $announcement->title }}</h3>
                    <p>{{ $announcement->body }}</p>
                    @if ($announcement->action_url !== null && $announcement->action_label !== null)
                        <a href="{{ $announcement->action_url }}"
                           @if (str_starts_with($announcement->action_url, 'https://')) rel="noopener noreferrer" @endif>
                            {{ $announcement->action_label }}
                        </a>
                    @endif
                </article>
            @endforeach
        </div>
    @elseif ($ticker->state === \App\Announcements\ViewModels\AnnouncementTickerState::EMPTY)
        <div class="empty-state">
            <strong>{{ __('public.announcements.empty') }}</strong>
            <p>{{ __('public.announcements.empty_help') }}</p>
        </div>
    @else
        <div class="alert alert-danger" role="status">
            {{ __('public.announcements.unavailable') }}
        </div>
    @endif
</section>
