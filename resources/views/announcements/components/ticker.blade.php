<section class="card" aria-labelledby="announcement-ticker-title">
    <div class="section-heading">
        <p class="eyebrow">Important notices</p>
        <h2 id="announcement-ticker-title">Announcements</h2>
    </div>

    @if ($ticker->state === \App\PublicPortal\PublicContentState::AVAILABLE)
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
    @elseif ($ticker->state === \App\PublicPortal\PublicContentState::EMPTY)
        <div class="empty-state">
            <strong>No active announcements.</strong>
            <p>Important notices will appear here during their approved publication window.</p>
        </div>
    @else
        <div class="alert alert-danger" role="status">
            Announcement information is temporarily unavailable.
        </div>
    @endif
</section>
