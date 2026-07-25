@extends('game.layout')

@section('title', __('public.news.title'))

@section('content')
    @inject('localeFormatter', 'App\Localization\LocaleFormatter')
    <div class="page-header">
        <p class="eyebrow">{{ __('public.news.eyebrow') }}</p>
        <h1>{{ __('public.news.title') }}</h1>
        <p class="muted">{{ __('public.news.description') }}</p>
    </div>

    @forelse ($posts as $post)
        <article class="card">
            <p class="eyebrow">{{ __('public.news.published', ['date' => $post->published_at ? $localeFormatter->dateTime($post->published_at) : '']) }}</p>
            <h2><a href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ $post->title }}</a></h2>
            <a href="{{ route('news.show', ['slug' => $post->slug]) }}">{{ __('public.news.read') }}</a>
        </article>
    @empty
        <div class="empty-state">
            <strong>{{ __('public.news.empty') }}</strong>
            <p>{{ __('public.news.empty_help') }}</p>
        </div>
    @endforelse

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
