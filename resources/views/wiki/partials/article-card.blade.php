<article class="card wiki-article-card">
    <h3><a href="{{ route('wiki.article', ['slug' => $articleCard->slug]) }}">{{ $articleCard->title }}</a></h3>
    <p>{{ $articleCard->summary }}</p>
    <p class="muted">{{ __('public.wiki.published', ['date' => $articleCard->publishedAt->toDateString()]) }}</p>
</article>
