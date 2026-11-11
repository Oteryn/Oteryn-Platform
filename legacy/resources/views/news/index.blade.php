@extends('game.layout')

@section('title', __('public.news.title'))
@section('portal-family', 'chronicles')

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.news.eyebrow') }}</p>
        <h1>{{ __('public.news.title') }}</h1>
        <p class="muted">{{ __('public.news.description') }}</p>
    </div>

    <div class="chronicle-index">
    @forelse ($posts as $post)
        <article class="card chronicle-entry @if($loop->first && $posts->onFirstPage()) chronicle-entry-lead @endif">
            @if ($loop->first && $posts->onFirstPage())
                <div class="chronicle-illustration" aria-hidden="true">
                    <img src="{{ asset('images/oteryn-citadel.webp') }}" width="626" height="468" alt="" decoding="async">
                </div>
            @endif
            <div class="chronicle-copy">
            <p class="eyebrow">{{ __('public.news.published', ['date' => $post->published_at ? $localeFormatter->dateTime($post->published_at) : '']) }}</p>
            <h2><a href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ $post->title }}</a></h2>
            <p class="chronicle-excerpt">{{ \Illuminate\Support\Str::limit(strip_tags($post->body), $loop->first ? 260 : 180) }}</p>
            <a class="text-link" href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ __('public.news.read') }} <span aria-hidden="true">→</span></a>
            </div>
        </article>
    @empty
        <div class="empty-state">
            <strong>{{ __('public.news.empty') }}</strong>
            <p>{{ __('public.news.empty_help') }}</p>
        </div>
    @endforelse
    </div>

    @if ($posts->hasPages())
        <nav class="pagination" aria-label="{{ __('public.news.pages') }}">
            @if ($posts->onFirstPage())
                <span class="muted">{{ __('public.pagination.previous') }}</span>
            @else
                <a href="{{ $posts->previousPageUrl() }}">{{ __('public.pagination.previous') }}</a>
            @endif
            <span>{{ __('public.pagination.page_of', ['current' => $posts->currentPage(), 'last' => $posts->lastPage()]) }}</span>
            @if ($posts->hasMorePages())
                <a href="{{ $posts->nextPageUrl() }}">{{ __('public.pagination.next') }}</a>
            @else
                <span class="muted">{{ __('public.pagination.next') }}</span>
            @endif
        </nav>
    @endif
@endsection
